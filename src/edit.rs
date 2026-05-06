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

/// Convert matched locations into edits for the given operation.
pub fn matches_to_edits(matches: &[Match], op: &Operation) -> Vec<Edit> {
    matches
        .iter()
        .map(|m| match op {
            Operation::Replace(replacement) => Edit {
                start_byte: m.start_byte,
                end_byte: m.end_byte,
                replacement: replacement.clone(),
            },
            Operation::Delete => Edit {
                start_byte: m.start_byte,
                end_byte: m.end_byte,
                replacement: String::new(),
            },
            Operation::InsertBefore(code) => Edit {
                start_byte: m.start_byte,
                end_byte: m.start_byte,
                replacement: code.clone(),
            },
            Operation::InsertAfter(code) => Edit {
                start_byte: m.end_byte,
                end_byte: m.end_byte,
                replacement: code.clone(),
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
        let source = "b";
        let edits = vec![Edit {
            start_byte: 0,
            end_byte: 0,
            replacement: "a".into(),
        }];
        assert_eq!(apply_edits(source, &edits).unwrap(), "ab");
    }

    #[test]
    fn test_insert_after() {
        let source = "a";
        let edits = vec![Edit {
            start_byte: 1,
            end_byte: 1,
            replacement: "b".into(),
        }];
        assert_eq!(apply_edits(source, &edits).unwrap(), "ab");
    }
}
