use std::collections::HashMap;
use tree_sitter::{
    Language, Node, Parser, Point, Query, QueryCursor, StreamingIterator, TreeCursor,
};

#[derive(Debug, Clone)]
pub struct Match {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: Point,
    #[allow(dead_code)]
    pub end_point: Point,
    /// Placeholder captures from snippet matching.
    /// Maps `$name` → the source text it matched.
    pub captures: HashMap<String, String>,
}

/// Metadata about a `$($name)sep*` repetition placeholder.
#[derive(Debug, Clone)]
pub(crate) struct RepetitionInfo {
    /// Repetition operator: "*" or "+"
    op: String,
}

fn is_root_wrapper(kind: &str) -> bool {
    matches!(kind, "program" | "module")
}

/// Extract the meaningful pattern node from a parsed snippet tree.
fn extract_pattern(root: Node) -> Option<Node> {
    if is_root_wrapper(root.kind()) && root.named_child_count() == 1 {
        root.named_child(0)
    } else {
        Some(root)
    }
}

/// Check if a node's subtree contains an ERROR node.
fn has_error(node: Node) -> bool {
    if node.kind() == "ERROR" {
        return true;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if has_error(cursor.node()) {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}

/// Check if a named leaf node matches by text content.
/// Named leaf nodes require exact text match by default.
fn leaf_matches(source_text: &str, pattern_text: &str, s_node: Node, p_node: Node) -> bool {
    let p = &pattern_text[p_node.start_byte()..p_node.end_byte()];
    let s = &source_text[s_node.start_byte()..s_node.end_byte()];
    p == s
}

/// Build reverse map from sentinel identifier to placeholder name.
/// `{"msg": "_pw_1"}` → `{"_pw_1": "msg"}`
fn reverse_placeholder_map(placeholders: &HashMap<String, String>) -> HashMap<String, String> {
    placeholders
        .iter()
        .map(|(k, v)| (v.clone(), k.clone()))
        .collect()
}

/// Check if a pattern node is a `$` placeholder (a bare `_pw_N` identifier).
fn is_placeholder(pattern_node: Node, pattern_text: &str) -> bool {
    if !pattern_node.is_named() || pattern_node.named_child_count() != 0 {
        return false;
    }
    let text = &pattern_text[pattern_node.start_byte()..pattern_node.end_byte()];
    text.starts_with("_pw_")
}

/// Check if the last named child of `parent` is a repetition placeholder.
/// Returns `Some((sentinel, info))` if so.
fn last_child_repetition(
    parent: Node,
    pattern_text: &str,
    repetitions: &HashMap<String, RepetitionInfo>,
) -> Option<(String, RepetitionInfo)> {
    let count = parent.named_child_count();
    if count == 0 {
        return None;
    }
    let last = parent.named_child(count as u32 - 1)?;
    if !is_placeholder(last, pattern_text) {
        return None;
    }
    let sentinel = pattern_text[last.start_byte()..last.end_byte()].to_string();
    repetitions
        .get(&sentinel)
        .map(|info| (sentinel, info.clone()))
}

/// Check if two nodes are structurally equivalent.
///
/// Named leaf nodes (identifiers, literals) match by text content by
/// default. Use `$name` in snippet patterns to match any single node
/// at that position. Use `$($name)sep*` / `$($name)sep+` to match
/// zero or more / one or more repetitions as the last child.
fn structurally_matches(
    source_node: Node,
    pattern_node: Node,
    source_text: &str,
    pattern_text: &str,
    captures: &mut HashMap<String, String>,
    reverse_placeholders: &HashMap<String, String>,
    repetitions: &HashMap<String, RepetitionInfo>,
) -> bool {
    // A $ placeholder matches any single source node (any kind, any text)
    if is_placeholder(pattern_node, pattern_text) {
        let sentinel = &pattern_text[pattern_node.start_byte()..pattern_node.end_byte()];
        if let Some(name) = reverse_placeholders.get(sentinel) {
            let matched = &source_text[source_node.start_byte()..source_node.end_byte()];
            captures.insert(name.clone(), matched.to_string());
        }
        return true;
    }

    if source_node.kind() != pattern_node.kind() {
        return false;
    }

    let pattern_named = pattern_node.named_child_count();
    let source_named = source_node.named_child_count();

    // Check if the last child of the *parent* is a repetition.
    // We handle this at the parent level by checking whether
    // `pattern_node` (a child of some caller) is a repetition.
    // Actually, we need to check if the pattern_node ITSELF
    // is the parent that has a repetition child. That happens
    // in the child comparison loop at the caller.
    //
    // But the repetition applies when the PATTERN node is the one
    // whose last child is a repetition. So we check here:
    if let Some((sentinel, rep)) = last_child_repetition(pattern_node, pattern_text, repetitions) {
        // Non-repetition children count = pattern_named - 1
        let fixed = pattern_named - 1;
        if source_named < fixed {
            return false;
        }
        let matched_count = source_named - fixed;

        // Check repetition bounds
        if rep.op == "+" && matched_count == 0 {
            return false;
        }
        if rep.op == "?" && matched_count > 1 {
            return false;
        }

        // Compare fixed children 1:1
        for i in 0..fixed {
            let Some(p_child) = pattern_node.named_child(i as u32) else {
                return false;
            };
            let Some(s_child) = source_node.named_child(i as u32) else {
                return false;
            };
            if !structurally_matches(
                s_child,
                p_child,
                source_text,
                pattern_text,
                captures,
                reverse_placeholders,
                repetitions,
            ) {
                return false;
            }
        }

        // Capture the repetition match
        if matched_count > 0 {
            if let Some(name) = reverse_placeholders.get(&sentinel) {
                let first = source_node.named_child(fixed as u32).unwrap();
                let last = source_node.named_child((source_named - 1) as u32).unwrap();
                let text = &source_text[first.start_byte()..last.end_byte()];
                captures.insert(name.clone(), text.to_string());
            }
        } else if let Some(name) = reverse_placeholders.get(&sentinel) {
            captures.insert(name.clone(), String::new());
        }

        return true;
    }

    // No repetition — exact child count match
    if pattern_named != source_named {
        return false;
    }

    // Both are named leaves — compare text
    if pattern_named == 0 {
        if pattern_node.is_named() {
            return leaf_matches(source_text, pattern_text, source_node, pattern_node);
        }
        return true;
    }

    // Compare named children in order
    for i in 0..pattern_named {
        let Some(p_child) = pattern_node.named_child(i as u32) else {
            return false;
        };
        let Some(s_child) = source_node.named_child(i as u32) else {
            return false;
        };
        if !structurally_matches(
            s_child,
            p_child,
            source_text,
            pattern_text,
            captures,
            reverse_placeholders,
            repetitions,
        ) {
            return false;
        }
    }

    true
}

/// If `pattern` is a single-child wrapper node (like `expression_statement`
/// wrapping an expression), return the inner child. This enables matching
/// through statement boundaries.
///
/// Skips unwrapping when the inner child is a bare placeholder or a leaf
/// (0 named children), both of which would match too broadly.
fn try_unwrap_pattern<'a>(pattern: Node<'a>, pattern_text: &'a str) -> Option<Node<'a>> {
    let count = pattern.named_child_count();
    if count == 1 {
        if let Some(child) = pattern.named_child(0) {
            if child.named_child_count() > 0 && !is_placeholder(child, pattern_text) {
                return Some(child);
            }
        }
    }
    None
}

/// Check if `node`'s byte range is entirely inside any match in `matches`.
/// Used to avoid duplicate matches from statement unwrapping.
fn overlaps_matched_range(node: Node, matches: &[Match]) -> bool {
    let start = node.start_byte();
    let end = node.end_byte();
    matches.iter().any(|m| {
        m.start_byte <= start && end <= m.end_byte && (m.start_byte < end || start < m.end_byte)
    })
}

/// Recursively traverse source tree collecting structural matches.
fn collect_matches(
    cursor: &mut TreeCursor,
    pattern: Node,
    matches: &mut Vec<Match>,
    source_text: &str,
    pattern_text: &str,
    reverse_placeholders: &HashMap<String, String>,
    repetitions: &HashMap<String, RepetitionInfo>,
) {
    let node = cursor.node();

    // Try full pattern match
    let mut captures = HashMap::new();
    if structurally_matches(
        node,
        pattern,
        source_text,
        pattern_text,
        &mut captures,
        reverse_placeholders,
        repetitions,
    ) {
        matches.push(Match {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: node.start_position(),
            end_point: node.end_position(),
            captures,
        });
    } else if !overlaps_matched_range(node, matches) {
        // Try unwrapped pattern — strip statement-level wrappers
        // to match deeper (e.g., method calls inside return_statements).
        // Skips bare placeholders to avoid overly broad matches.
        if let Some(inner) = try_unwrap_pattern(pattern, pattern_text) {
            let mut captures = HashMap::new();
            if structurally_matches(
                node,
                inner,
                source_text,
                pattern_text,
                &mut captures,
                reverse_placeholders,
                repetitions,
            ) {
                matches.push(Match {
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    start_point: node.start_position(),
                    end_point: node.end_position(),
                    captures,
                });
            }
        }
    }

    if cursor.goto_first_child() {
        loop {
            collect_matches(
                cursor,
                pattern,
                matches,
                source_text,
                pattern_text,
                reverse_placeholders,
                repetitions,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Preprocess a snippet, replacing `$name` placeholders with unique
/// sentinel identifiers so tree-sitter can parse them.
///
/// Also handles Rust-style repetition syntax:
/// - `$($name),*` — zero or more comma-separated
/// - `$($name),+` — one or more comma-separated
///
/// Returns the processed snippet text, a map of placeholder names,
/// and a map of sentinels with repetition info.
fn preprocess_snippet(
    snippet: &str,
) -> (
    String,
    HashMap<String, String>,
    HashMap<String, RepetitionInfo>,
) {
    let mut result = String::with_capacity(snippet.len());
    let mut placeholders: HashMap<String, String> = HashMap::new();
    let mut repetitions: HashMap<String, RepetitionInfo> = HashMap::new();
    let mut counter = 0usize;
    let mut chars = snippet.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        if c == '$' {
            if let Some(&(_, next)) = chars.peek() {
                if next == '(' {
                    // Rust-style repetition: $($name)sep*  or  $($name)sep+
                    chars.next(); // consume '('

                    // Read inner content until matching ')'
                    let mut inner = String::new();
                    let mut depth = 1;
                    let mut balanced = false;
                    while let Some(&(_, ic)) = chars.peek() {
                        if ic == '(' {
                            depth += 1;
                            inner.push(ic);
                            chars.next();
                        } else if ic == ')' {
                            depth -= 1;
                            chars.next();
                            if depth == 0 {
                                balanced = true;
                                break;
                            }
                            inner.push(ic);
                        } else {
                            inner.push(ic);
                            chars.next();
                        }
                    }

                    if !balanced {
                        // Unmatched parens — treat as literal
                        result.push_str("$(");
                        result.push_str(&inner);
                        continue;
                    }

                    // Read separator (anything between ')' and operator)
                    let mut sep = String::new();
                    while let Some(&(_, sc)) = chars.peek() {
                        if sc == '*' || sc == '+' || sc == '?' {
                            break;
                        }
                        sep.push(sc);
                        chars.next();
                    }

                    // Read operator
                    let op = match chars.next() {
                        Some((_, '*')) => "*".to_string(),
                        Some((_, '+')) => "+".to_string(),
                        Some((_, '?')) => "?".to_string(),
                        _ => {
                            // Invalid repetition syntax — treat as literal
                            result.push_str("$(");
                            result.push_str(&inner);
                            result.push_str(&sep);
                            continue;
                        }
                    };

                    // Extract inner placeholder: $name (ignore any trailing content like commas)
                    let inner_trimmed = inner.trim();
                    let name = if let Some(rest) = inner_trimmed.strip_prefix('$') {
                        let mut cleaned = String::new();
                        for ch in rest.trim().chars() {
                            if ch.is_ascii_alphanumeric() || ch == '_' {
                                cleaned.push(ch);
                            } else {
                                break;
                            }
                        }
                        cleaned
                    } else {
                        String::new()
                    };

                    if name.is_empty() {
                        // Not valid repetition syntax — treat as literal
                        result.push_str("$(");
                        result.push_str(&inner);
                        result.push_str(&sep);
                        result.push_str(&op);
                        continue;
                    }

                    if let Some(existing) = placeholders.get(&name) {
                        result.push_str(existing);
                        repetitions.insert(existing.clone(), RepetitionInfo { op });
                    } else {
                        counter += 1;
                        let sentinel = format!("_pw_{}", counter);
                        placeholders.insert(name, sentinel.clone());
                        repetitions.insert(sentinel.clone(), RepetitionInfo { op });
                        result.push_str(&sentinel);
                    }
                    continue;
                } else if next.is_ascii_alphabetic() || next == '_' {
                    let mut name = String::new();
                    while let Some(&(_, c)) = chars.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            name.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Some(existing) = placeholders.get(&name) {
                        result.push_str(existing);
                    } else {
                        counter += 1;
                        let sentinel = format!("_pw_{}", counter);
                        placeholders.insert(name, sentinel.clone());
                        result.push_str(&sentinel);
                    }
                    continue;
                }
            }
        }
        result.push(c);
    }

    (result, placeholders, repetitions)
}

/// Find all snippet matches in source text.
///
/// Named nodes (identifiers, literals) are matched by text content by
/// default. Prefix an identifier with `$` (e.g. `$x`) to match any
/// value at that position.
///
/// Use `$($name)sep*` / `$($name)sep+` (Rust macro repetition syntax)
/// for zero-or-more (`*`), one-or-more (`+`), or optional (`?`) matching
/// in the last child position.
pub fn find_snippet_matches(
    source: &str,
    snippet: &str,
    lang: &Language,
) -> Result<Vec<Match>, String> {
    let (pattern_text, placeholders, repetitions) = preprocess_snippet(snippet);
    let reverse_ph = reverse_placeholder_map(&placeholders);

    let mut parser = Parser::new();
    parser
        .set_language(lang)
        .map_err(|e| format!("Failed to set language: {}", e))?;

    let snippet_tree = parser
        .parse(&pattern_text, None)
        .ok_or("Failed to parse snippet")?;
    let pattern = extract_pattern(snippet_tree.root_node())
        .ok_or("Could not extract pattern from snippet")?;

    if has_error(pattern) {
        return Err("Snippet contains syntax errors".to_string());
    }

    let source_tree = parser.parse(source, None).ok_or("Failed to parse source")?;

    let mut matches = Vec::new();
    let mut cursor = source_tree.root_node().walk();
    collect_matches(
        &mut cursor,
        pattern,
        &mut matches,
        source,
        &pattern_text,
        &reverse_ph,
        &repetitions,
    );
    Ok(matches)
}

/// Find matches using a tree-sitter S-expression query.
pub fn find_query_matches(
    source: &str,
    query_str: &str,
    lang: &Language,
) -> Result<Vec<Match>, String> {
    let query = Query::new(lang, query_str)
        .map_err(|e| format!("Query error at {}:{}: {}", e.row, e.column, e.message))?;

    let mut parser = Parser::new();
    parser
        .set_language(lang)
        .map_err(|_| "Failed to set language".to_string())?;
    let source_tree = parser.parse(source, None).ok_or("Failed to parse source")?;

    let mut qc = QueryCursor::new();
    let mut query_matches = qc.matches(&query, source_tree.root_node(), source.as_bytes());

    let capture_names: Vec<&str> = query.capture_names().iter().map(|s| s.as_ref()).collect();
    let mut results = Vec::new();

    while let Some(qm) = query_matches.next() {
        if qm.captures.is_empty() {
            continue;
        }

        let target = qm
            .captures
            .iter()
            .find(|c| {
                capture_names
                    .get(c.index as usize)
                    .map(|n| *n == "matched" || *n == "target")
                    .unwrap_or(false)
            })
            .unwrap_or(&qm.captures[0]);

        let node = target.node;
        results.push(Match {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: node.start_position(),
            end_point: node.end_position(),
            captures: HashMap::new(),
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn java_lang() -> Language {
        tree_sitter_java::LANGUAGE.into()
    }

    fn python_lang() -> Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn js_lang() -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    // ——— by default, leaf nodes match by name ———

    #[test]
    fn test_exact_name_match() {
        // Identifiers must match exactly by default
        let source = "class A { void f() { x = 1; } }";
        let matches = find_snippet_matches(source, "x = 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_identifier_name_mismatch() {
        // Different identifier name, no match
        let source = "class A { void f() { y = 1; } }";
        let matches = find_snippet_matches(source, "x = 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_literal_value_mismatch() {
        // Different integer value, no match
        let source = "class A { void f() { return 42; } }";
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_string_literal_mismatch() {
        let source = "class A { void f() { s = \"hello\"; } }";
        let matches = find_snippet_matches(source, "s = \"world\";", &java_lang()).unwrap();
        assert_eq!(matches.len(), 0);
    }

    // ——— $ placeholder wildcard matching ———

    #[test]
    fn test_dollar_placeholder_matches_any_identifier() {
        let source = "class A { void f() { println(42); } }";
        let matches = find_snippet_matches(source, "$method(42);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_dollar_placeholder_matches_any_literal() {
        let source = "class A { void f() { return 42; } }";
        let matches = find_snippet_matches(source, "return $n;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_dollar_placeholder_matches_any_string() {
        let source = "class A { void f() { s = \"hello\"; } }";
        let matches = find_snippet_matches(source, "s = $s;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_dollar_multiple_placeholders() {
        let source = "class A { void f() { foo(1, 2); } }";
        let matches = find_snippet_matches(source, "$f($a, $b);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_multiple_matches_with_placeholder() {
        let source = r#"
class A {
    void f() { return 1; }
    void g() { return 2; }
    void h() { return 3; }
}"#;
        let matches = find_snippet_matches(source, "return $n;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 3);
    }

    // ——— existing test cases adapted for name-matched default ———

    #[test]
    fn test_same_kind_leaves_match() {
        let source = "class A { void f() { return 1; } }";
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_different_structure_no_match() {
        let source = "class A { void f() { if (true) {} } }";
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_snippet_collects_matches() {
        let source = r#"
class A {
    void f() {
        System.out.println("hello");
        System.out.println("world");
    }
}"#;
        let matches =
            find_snippet_matches(source, r#"System.out.println("hello");"#, &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_no_match_for_absent_code() {
        let source = "class A { void f() { return 1; } }";
        let matches =
            find_snippet_matches(source, "System.out.println(\"x\");", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_empty_source_no_match() {
        let matches = find_snippet_matches("", "return 1;", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_deeply_nested_match() {
        let source = r#"
class Outer {
    class Inner {
        void f() {
            if (true) {
                return 1;
            }
        }
    }
}"#;
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_match_byte_ranges_correct() {
        let source = "class A { int x; void f() { return 1; } }";
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        let m = &matches[0];
        assert_eq!(&source[m.start_byte..m.end_byte], "return 1;");
    }

    #[test]
    fn test_snippet_error_returns_err() {
        let result = find_snippet_matches("class A {}", "return ", &java_lang());
        assert!(result.is_err());
    }

    // ——— Python snippet matching ———

    #[test]
    fn test_python_snippet_match() {
        let source = "x = 42\n";
        let matches = find_snippet_matches(source, "x = 42", &python_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_python_multiple_matches() {
        let source = "a = 1\nb = 2\n";
        let matches = find_snippet_matches(source, "$x = $n", &python_lang()).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_python_no_match() {
        let source = "def f():\n    pass\n";
        let matches = find_snippet_matches(source, "return 1", &python_lang()).unwrap();
        assert!(matches.is_empty());
    }

    // ——— JavaScript snippet matching ———

    #[test]
    fn test_js_snippet_match() {
        let source = "function f() { return 42; }";
        let matches = find_snippet_matches(source, "return 42;", &js_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_js_no_match() {
        let source = "const x = 1;";
        let matches = find_snippet_matches(source, "return 1;", &js_lang()).unwrap();
        assert!(matches.is_empty());
    }

    // ——— query matching ———

    #[test]
    fn test_query_find() {
        let source = "class A { void f() { return 42; } }";
        let matches =
            find_query_matches(source, "(return_statement) @matched", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_query_no_matches() {
        let source = "class A { void f() { return 42; } }";
        let matches = find_query_matches(source, "(if_statement) @matched", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_query_prefers_matched_capture() {
        let source = "class A { void f() { return 42; } }";
        let matches = find_query_matches(
            source,
            "(method_declaration name: (identifier) @matched)",
            &java_lang(),
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(&source[matches[0].start_byte..matches[0].end_byte], "f");
    }

    #[test]
    fn test_query_syntax_error() {
        let result = find_query_matches("class A {}", "((", &java_lang());
        assert!(result.is_err());
    }

    #[test]
    fn test_python_query() {
        let source = "def f():\n    return 42\n";
        let matches =
            find_query_matches(source, "(return_statement) @matched", &python_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    // ——— structurally_matches ———

    #[test]
    fn test_structurally_matches_fails_on_kind_mismatch() {
        let source = "class A { void f() { return 1; } }";
        let matches = find_snippet_matches(source, "if (true) {}", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    // ——— preprocess_snippet ———

    #[test]
    fn test_preprocess_replaces_dollar() {
        let (text, _, _) = preprocess_snippet("$x = $y;");
        assert_eq!(text, "_pw_1 = _pw_2;");
    }

    #[test]
    fn test_preprocess_no_dollar() {
        let (text, _, _) = preprocess_snippet("x = y;");
        assert_eq!(text, "x = y;");
    }

    // ——— repetition syntax ———

    #[test]
    fn test_preprocess_repetition_star() {
        let (text, placeholders, reps) = preprocess_snippet("$f($($arg,)*)");
        // Should produce: _pw_1(_pw_2) with _pw_2 marked as repetition "*"
        assert_eq!(text, "_pw_1(_pw_2)");
        assert_eq!(placeholders.get("f").unwrap(), "_pw_1");
        assert_eq!(placeholders.get("arg").unwrap(), "_pw_2");
        assert_eq!(reps.get("_pw_2").unwrap().op, "*");
    }

    #[test]
    fn test_preprocess_repetition_plus() {
        let (_, _, reps) = preprocess_snippet("$($arg,)+");
        assert!(reps.get("_pw_1").is_some());
        assert_eq!(reps.get("_pw_1").unwrap().op, "+");
    }

    #[test]
    fn test_repetition_matches_zero_args() {
        // $f($($arg,)*) should match f() (zero args)
        let source = "class A { void m() { f(); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)*);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        let m = &matches[0];
        // $arg should capture empty string
        assert_eq!(m.captures.get("arg").map(|s| s.as_str()), Some(""));
    }

    #[test]
    fn test_repetition_matches_one_arg() {
        // $f($($arg,)*) should match f(x) (one arg)
        let source = "class A { void m() { f(x); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)*);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        let m = &matches[0];
        assert_eq!(m.captures.get("arg").map(|s| s.as_str()), Some("x"));
    }

    #[test]
    fn test_repetition_matches_multiple_args() {
        // $f($($arg,)*) should match f(x, y, z) (three args)
        let source = "class A { void m() { f(x, y, z); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)*);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        let m = &matches[0];
        assert_eq!(m.captures.get("f").unwrap(), "f");
        // Captured text spans all args including separators
        assert!(m.captures.get("arg").unwrap().len() >= 3);
        assert!(m.captures.get("arg").unwrap().contains('x'));
        assert!(m.captures.get("arg").unwrap().contains('y'));
        assert!(m.captures.get("arg").unwrap().contains('z'));
    }

    #[test]
    fn test_repetition_plus_requires_at_least_one() {
        // $f($($arg,)+) should NOT match f() (zero args)
        let source = "class A { void m() { f(); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)+);", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_repetition_plus_matches_one() {
        // $f($($arg,)+) should match f(x) (one arg)
        let source = "class A { void m() { f(x); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)+);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    // ——— statement unwrapping ———

    #[test]
    fn test_unwrap_expression_in_return_statement() {
        // Pattern `users.$method()` (expression_statement) should match
        // a method call inside a return statement via unwrapping
        let source = "class A { void m() { return users.size(); } }";
        // With repetition: $($arg,)* so () matches zero args
        let matches =
            find_snippet_matches(source, "users.$method($($arg,)*);", &java_lang()).unwrap();
        assert_eq!(
            matches.len(),
            1,
            "should match return users.size() via unwrapping"
        );
        let m = &matches[0];
        assert_eq!(m.captures.get("method").unwrap(), "size");
    }

    #[test]
    fn test_unwrap_expression_in_variable_declaration() {
        // Pattern `users.$method()` should match
        // a method call inside a variable declaration
        let source = "class A { void m() { String s = users.get(id); } }";
        let matches =
            find_snippet_matches(source, "users.$method($($arg,)*);", &java_lang()).unwrap();
        assert_eq!(
            matches.len(),
            1,
            "should match users.get(id) in variable decl"
        );
        assert_eq!(matches[0].captures.get("method").unwrap(), "get");
    }

    #[test]
    fn test_unwrap_no_duplicate_with_direct_match() {
        // When the pattern directly matches an expression_statement,
        // we should NOT also get a duplicate inner match
        let source = "class A { void m() { users.size(); } }";
        let matches =
            find_snippet_matches(source, "users.$method($($arg,)*);", &java_lang()).unwrap();
        // Should get exactly 1 match (the expression_statement)
        assert_eq!(matches.len(), 1);
    }

    // ——— combined: $method() matching on UserService.java ———

    #[test]
    fn test_method_with_any_args_matches_all_forms() {
        // The original failing case: users.$method() should match
        // all method calls on users regardless of arguments
        let source = r#"
class A {
    void f() {
        users.size();
        String s = users.get(id);
        return users.remove(id);
    }
}"#;
        let matches =
            find_snippet_matches(source, "users.$method($($arg,)*);", &java_lang()).unwrap();
        // Should match all three: users.size(), users.get(id), users.remove(id)
        assert_eq!(matches.len(), 3, "should match all three forms");
    }

    // ——— optional repetition (?) ———

    #[test]
    fn test_repetition_optional_matches_zero() {
        // $f($($arg,)?) should match f() (zero args)
        let source = "class A { void m() { f(); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)?);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].captures.get("arg").map(|s| s.as_str()), Some(""));
    }

    #[test]
    fn test_repetition_optional_matches_one() {
        // $f($($arg,)?) should match f(x) (one arg)
        let source = "class A { void m() { f(x); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)?);", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].captures.get("arg").map(|s| s.as_str()), Some("x"));
    }

    #[test]
    fn test_repetition_optional_rejects_two() {
        // $f($($arg,)?) should NOT match f(x, y) (two args)
        let source = "class A { void m() { f(x, y); } }";
        let matches = find_snippet_matches(source, "$f($($arg,)?);", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }
}
