use tree_sitter::Language;

pub enum Lang {
    Java,
    Python,
    JavaScript,
    TypeScript,
    TSX,
}

impl Lang {
    pub fn from_extension(path: &str) -> Option<Lang> {
        let ext = std::path::Path::new(path)
            .extension()?
            .to_str()?
            .to_lowercase();
        match ext.as_str() {
            "java" => Some(Lang::Java),
            "py" => Some(Lang::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Lang::JavaScript),
            "ts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::TSX),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Lang> {
        match name.to_lowercase().as_str() {
            "java" => Some(Lang::Java),
            "python" | "py" => Some(Lang::Python),
            "javascript" | "js" => Some(Lang::JavaScript),
            "typescript" | "ts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::TSX),
            _ => None,
        }
    }

    pub fn grammar(&self) -> Language {
        match self {
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::TSX => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_mapping() {
        assert!(matches!(Lang::from_extension("foo.java"), Some(Lang::Java)));
        assert!(matches!(Lang::from_extension("foo.py"), Some(Lang::Python)));
        assert!(matches!(
            Lang::from_extension("foo.js"),
            Some(Lang::JavaScript)
        ));
        assert!(matches!(
            Lang::from_extension("foo.ts"),
            Some(Lang::TypeScript)
        ));
        assert!(matches!(Lang::from_extension("foo.tsx"), Some(Lang::TSX)));
        assert!(Lang::from_extension("foo.rs").is_none());
    }

    #[test]
    fn test_name_mapping() {
        assert!(matches!(Lang::from_name("java"), Some(Lang::Java)));
        assert!(matches!(Lang::from_name("python"), Some(Lang::Python)));
        assert!(matches!(Lang::from_name("py"), Some(Lang::Python)));
        assert!(matches!(Lang::from_name("js"), Some(Lang::JavaScript)));
        assert!(matches!(Lang::from_name("ts"), Some(Lang::TypeScript)));
        assert!(Lang::from_name("ruby").is_none());
    }

    #[test]
    fn test_additional_extensions() {
        assert!(matches!(
            Lang::from_extension("test.jsx"),
            Some(Lang::JavaScript)
        ));
        assert!(matches!(
            Lang::from_extension("test.mjs"),
            Some(Lang::JavaScript)
        ));
        assert!(matches!(
            Lang::from_extension("test.cjs"),
            Some(Lang::JavaScript)
        ));
    }

    #[test]
    fn test_grammar_loads() {
        for lang in &[Lang::Java, Lang::Python, Lang::JavaScript] {
            let grammar = lang.grammar();
            let mut p = tree_sitter::Parser::new();
            assert!(p.set_language(&grammar).is_ok());
        }
    }

    #[test]
    fn test_case_insensitive_extension() {
        assert!(matches!(
            Lang::from_extension("TEST.JAVA"),
            Some(Lang::Java)
        ));
        assert!(matches!(
            Lang::from_extension("Test.Py"),
            Some(Lang::Python)
        ));
    }
}
