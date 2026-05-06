use tree_sitter::{Language, Node, Parser, Point, Query, QueryCursor, StreamingIterator, TreeCursor};

#[derive(Debug, Clone)]
pub struct Match {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: Point,
    #[allow(dead_code)]
    pub end_point: Point,
}

fn is_root_wrapper(kind: &str) -> bool {
    matches!(kind, "program" | "module")
}

/// Extract the meaningful pattern node from a parsed snippet tree.
/// Skips the root wrapper (program/module) if the snippet is a single
/// top-level construct.
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

/// Check if two nodes are structurally equivalent.
///
/// Two nodes match if they have the same kind and each named child
/// recursively matches. Leaf values (identifiers, literals) are compared
/// by kind only, not by text content — any `string_literal` matches any
/// other `string_literal`.
pub fn structurally_matches(source_node: Node, pattern_node: Node) -> bool {
    if source_node.kind() != pattern_node.kind() {
        return false;
    }

    let pattern_named = pattern_node.named_child_count();
    let source_named = source_node.named_child_count();

    if pattern_named != source_named {
        return false;
    }

    // Both are leaves and kinds already match
    if pattern_named == 0 {
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
        if !structurally_matches(s_child, p_child) {
            return false;
        }
    }

    true
}

/// Recursively traverse source tree collecting structural matches.
fn collect_matches(cursor: &mut TreeCursor, pattern: Node, matches: &mut Vec<Match>) {
    let node = cursor.node();
    if structurally_matches(node, pattern) {
        matches.push(Match {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: node.start_position(),
            end_point: node.end_position(),
        });
    }

    if cursor.goto_first_child() {
        loop {
            collect_matches(cursor, pattern, matches);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Find all snippet matches in source text.
pub fn find_snippet_matches(
    source: &str,
    snippet: &str,
    lang: &Language,
) -> Result<Vec<Match>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(lang)
        .map_err(|e| format!("Failed to set language: {}", e))?;

    let snippet_tree = parser
        .parse(snippet, None)
        .ok_or("Failed to parse snippet")?;
    let pattern =
        extract_pattern(snippet_tree.root_node()).ok_or("Could not extract pattern from snippet")?;

    if has_error(pattern) {
        return Err("Snippet contains syntax errors".to_string());
    }

    let source_tree = parser
        .parse(source, None)
        .ok_or("Failed to parse source")?;

    let mut matches = Vec::new();
    let mut cursor = source_tree.root_node().walk();
    collect_matches(&mut cursor, pattern, &mut matches);
    Ok(matches)
}

/// Find matches using a tree-sitter S-expression query.
pub fn find_query_matches(
    source: &str,
    query_str: &str,
    lang: &Language,
) -> Result<Vec<Match>, String> {
    let query =
        Query::new(lang, query_str).map_err(|e| {
            format!(
                "Query error at {}:{}: {}",
                e.row, e.column, e.message
            )
        })?;

    let mut parser = Parser::new();
    parser
        .set_language(lang)
        .map_err(|_| "Failed to set language".to_string())?;
    let source_tree = parser
        .parse(source, None)
        .ok_or("Failed to parse source")?;

    let mut qc = QueryCursor::new();
    let mut query_matches = qc.matches(&query, source_tree.root_node(), source.as_bytes());

    let capture_names: Vec<&str> = query.capture_names().iter().map(|s| s.as_ref()).collect();
    let mut results = Vec::new();

    while let Some(qm) = query_matches.next() {
        if qm.captures.is_empty() {
            continue;
        }

        // Prefer @matched or @target capture; fall back to first capture
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

    // ——— snippet matching ———

    #[test]
    fn test_same_kind_leaves_match() {
        let source = "class A { void f() { return 1; } }";
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_leaf_value_ignored() {
        let source = "class A { void f() { return 42; } }";
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_different_structure_no_match() {
        let source = "class A { void f() { if (a) {} } }";
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
            find_snippet_matches(source, r#"System.out.println("...");"#, &java_lang()).unwrap();
        assert_eq!(matches.len(), 2);
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
    fn test_multiple_matches() {
        let source = r#"
class A {
    void f() { return 1; }
    void g() { return 2; }
    void h() { return 3; }
}"#;
        let matches = find_snippet_matches(source, "return 1;", &java_lang()).unwrap();
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_snippet_error_returns_err() {
        let result = find_snippet_matches("class A {}", "return ", &java_lang());
        assert!(result.is_err());
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
        let matched_text = &source[m.start_byte..m.end_byte];
        assert_eq!(matched_text, "return 1;");
    }

    // ——— Python snippet matching ———

    #[test]
    fn test_python_snippet_match() {
        let source = "x = 42\n";
        let matches = find_snippet_matches(source, "x = 1", &python_lang()).unwrap();
        assert_eq!(matches.len(), 1);
        // The match may include trailing newline; check it starts correctly
        assert!(matches[0].end_byte >= 6);
        assert!(source[matches[0].start_byte..].starts_with("x = 42"));
    }

    #[test]
    fn test_python_multiple_matches() {
        let source = "a = 1\nb = 2\n";
        let matches = find_snippet_matches(source, "a = 1", &python_lang()).unwrap();
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
        let matches = find_snippet_matches(source, "return 1;", &js_lang()).unwrap();
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
        let matches =
            find_query_matches(source, "(if_statement) @matched", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_query_prefers_matched_capture() {
        let source = "class A { void f() { return 42; } }";
        // Query captures a method_declaration's identifier — should use @matched
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
        // An if-statement doesn't match a return statement
        let source = "class A { void f() { return 1; } }";
        let matches = find_snippet_matches(source, "if (true) {}", &java_lang()).unwrap();
        assert!(matches.is_empty());
    }
}
