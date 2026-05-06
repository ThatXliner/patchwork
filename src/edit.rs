use std::collections::HashMap;
use crate::matcher::Match;

#[derive(Debug, Clone)]
pub struct Edit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

pub enum Operation {
    Replace(String),
    Delete,
    InsertBefore(String),
    InsertAfter(String),
}

/// Substitute `$name` references in `template` with values from `captures`.
/// Unknown `$name` is left as-is.
fn substitute_captures(template: &str, captures: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        if c == '$' {
            if let Some(&(_, next)) = chars.peek() {
                if next.is_ascii_alphabetic() || next == '_' {
                    let mut name = String::new();
                    while let Some(&(_, c)) = chars.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            name.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Some(value) = captures.get(&name) {
                        result.push_str(value);
                    } else {
                        result.push('$');
                        result.push_str(&name);
                    }
                    continue;
                }
            }
        }
        result.push(c);
    }

    result
}

/// Convert matched locations into edits for the given operation.
pub fn matches_to_edits(matches: &[Match], op: &Operation) -> Vec<Edit> {
    matches
        .iter()
        .map(|m| match op {
            Operation::Replace(replacement) => Edit {
                start_byte: m.start_byte,
                end_byte: m.end_byte,
                replacement: substitute_captures(replacement, &m.captures),
            },
            Operation::Delete => Edit {
                start_byte: m.start_byte,
                end_byte: m.end_byte,
                replacement: String::new(),
            },
            Operation::InsertBefore(code) => Edit {
                start_byte: m.start_byte,
                end_byte: m.start_byte,
                replacement: substitute_captures(code, &m.captures),
            },
            Operation::InsertAfter(code) => Edit {
                start_byte: m.end_byte,
                end_byte: m.end_byte,
                replacement: substitute_captures(code, &m.captures),
            },
        })
        .collect()
}

/// Apply edits to source text, processing bottom-up to preserve byte offsets.
pub fn apply_edits(source: &str, edits: &[Edit]) -> Result<String, String> {
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

    // Check for overlapping edits
    for i in 1..sorted.len() {
        if sorted[i].end_byte > sorted[i - 1].start_byte {
            return Err(format!(
                "Overlapping edits detected at byte {}..{}: matches overlap, refine your pattern",
                sorted[i].start_byte, sorted[i].end_byte
            ));
        }
    }

    let mut result = source.to_string();
    for edit in &sorted {
        result.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use tree_sitter::Point;
    use super::*;

    #[test]
    fn test_replace() {
        let edits = vec![Edit {
            start_byte: 0,
            end_byte: 5,
            replacement: "hello".into(),
        }];
        assert_eq!(apply_edits("world", &edits).unwrap(), "hello");
    }

    #[test]
    fn test_delete() {
        let edits = vec![Edit {
            start_byte: 5,
            end_byte: 11,
            replacement: String::new(),
        }];
        // " world" removed from "hello world"
        assert_eq!(apply_edits("hello world", &edits).unwrap(), "hello");
    }

    #[test]
    fn test_bottom_up_order() {
        // Edits in wrong order — should still work
        let edits = vec![
            Edit {
                start_byte: 3,
                end_byte: 4,
                replacement: "Y".into(),
            },
            Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: "X".into(),
            },
        ];
        // "ab cd": replace byte 0-1 with X → "Xb cd", byte 3-4 with Y → "Xb Yd"
        assert_eq!(apply_edits("ab cd", &edits).unwrap(), "Xb Yd");
    }

    #[test]
    fn test_overlapping_edits_error() {
        let edits = vec![
            Edit {
                start_byte: 0,
                end_byte: 5,
                replacement: "x".into(),
            },
            Edit {
                start_byte: 3,
                end_byte: 8,
                replacement: "y".into(),
            },
        ];
        assert!(apply_edits("hello world", &edits).is_err());
    }

    #[test]
    fn test_insert_before() {
        let edits = vec![Edit {
            start_byte: 0,
            end_byte: 0,
            replacement: "a".into(),
        }];
        assert_eq!(apply_edits("b", &edits).unwrap(), "ab");
    }

    #[test]
    fn test_insert_after() {
        let edits = vec![Edit {
            start_byte: 1,
            end_byte: 1,
            replacement: "b".into(),
        }];
        assert_eq!(apply_edits("a", &edits).unwrap(), "ab");
    }

    #[test]
    fn test_empty_source() {
        let edits = vec![Edit {
            start_byte: 0,
            end_byte: 0,
            replacement: "hello".into(),
        }];
        assert_eq!(apply_edits("", &edits).unwrap(), "hello");
    }

    #[test]
    fn test_replace_longer() {
        let edits = vec![Edit {
            start_byte: 0,
            end_byte: 1,
            replacement: "abc".into(),
        }];
        assert_eq!(apply_edits("x", &edits).unwrap(), "abc");
    }

    #[test]
    fn test_replace_shorter() {
        let edits = vec![Edit {
            start_byte: 0,
            end_byte: 3,
            replacement: "x".into(),
        }];
        assert_eq!(apply_edits("abc", &edits).unwrap(), "x");
    }

    #[test]
    fn test_multiple_non_overlapping() {
        let edits = vec![
            Edit {
                start_byte: 6,
                end_byte: 11,
                replacement: "there".into(),
            },
            Edit {
                start_byte: 0,
                end_byte: 5,
                replacement: "hi".into(),
            },
        ];
        assert_eq!(apply_edits("hello world", &edits).unwrap(), "hi there");
    }

    #[test]
    fn test_two_inserts_same_position() {
        let edits = vec![
            Edit {
                start_byte: 0,
                end_byte: 0,
                replacement: "b".into(),
            },
            Edit {
                start_byte: 0,
                end_byte: 0,
                replacement: "a".into(),
            },
        ];
        let result = apply_edits("", &edits).unwrap();
        assert_eq!(result, "ab");
    }

    #[test]
    fn test_utf8_multi_byte() {
        let edits = vec![Edit {
            start_byte: 3,
            end_byte: 4,
            replacement: "ñ".into(),
        }];
        // "héllo": h=0, é=1-2, l=3, l=4, o=5
        // Replace byte 3..4 (first 'l') with "ñ" → "héñlo"
        assert_eq!(apply_edits("héllo", &edits).unwrap(), "héñlo");
    }

    #[test]
    fn test_no_edits() {
        assert_eq!(apply_edits("hello", &[]).unwrap(), "hello");
    }

    #[test]
    fn test_insert_at_end() {
        let edits = vec![Edit {
            start_byte: 5,
            end_byte: 5,
            replacement: "!".into(),
        }];
        assert_eq!(apply_edits("hello", &edits).unwrap(), "hello!");
    }

    #[test]
    fn test_matches_to_edits_replace() {
        let m = Match {
            start_byte: 0,
            end_byte: 5,
            start_point: Point { row: 0, column: 0 },
            end_point: Point { row: 0, column: 5 },
            captures: HashMap::new(),
        };
        let edits = matches_to_edits(&[m], &Operation::Replace("hi".into()));
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start_byte, 0);
        assert_eq!(edits[0].end_byte, 5);
        assert_eq!(edits[0].replacement, "hi");
    }

    #[test]
    fn test_matches_to_edits_delete() {
        let m = Match {
            start_byte: 1,
            end_byte: 4,
            start_point: Point { row: 0, column: 1 },
            end_point: Point { row: 0, column: 4 },
            captures: HashMap::new(),
        };
        let edits = matches_to_edits(&[m], &Operation::Delete);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start_byte, 1);
        assert_eq!(edits[0].end_byte, 4);
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_substitute_captures_basic() {
        let mut caps = HashMap::new();
        caps.insert("msg".into(), "\"hello\"".into());
        assert_eq!(substitute_captures("warn($msg)", &caps), "warn(\"hello\")");
    }

    #[test]
    fn test_substitute_captures_multiple() {
        let mut caps = HashMap::new();
        caps.insert("f".into(), "foo".into());
        caps.insert("a".into(), "1".into());
        caps.insert("b".into(), "2".into());
        assert_eq!(
            substitute_captures("$f($b, $a)", &caps),
            "foo(2, 1)"
        );
    }

    #[test]
    fn test_substitute_captures_unknown_kept_as_is() {
        let caps = HashMap::new();
        assert_eq!(substitute_captures("warn($msg)", &caps), "warn($msg)");
    }

    #[test]
    fn test_substitute_captures_no_dollar() {
        let caps = HashMap::new();
        assert_eq!(substitute_captures("hello world", &caps), "hello world");
    }

    #[test]
    fn test_matches_to_edits_with_captures() {
        let mut caps = HashMap::new();
        caps.insert("msg".into(), "\"hello\"".into());
        let m = Match {
            start_byte: 24,
            end_byte: 42,
            start_point: Point { row: 2, column: 0 },
            end_point: Point { row: 2, column: 18 },
            captures: caps,
        };
        let edits = matches_to_edits(&[m], &Operation::Replace("warn($msg)".into()));
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "warn(\"hello\")");
    }
}
