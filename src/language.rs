use tree_sitter::Language;

pub enum Lang {
    Java,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Rust,
    Go,
    Ruby,
    C,
    Cpp,
    CSharp,
    Php,
    Bash,
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
            "tsx" => Some(Lang::Tsx),
            "rs" => Some(Lang::Rust),
            "go" => Some(Lang::Go),
            "rb" => Some(Lang::Ruby),
            "c" => Some(Lang::C),
            "h" => Some(Lang::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => Some(Lang::Cpp),
            "cs" => Some(Lang::CSharp),
            "php" => Some(Lang::Php),
            "sh" | "bash" | "zsh" => Some(Lang::Bash),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Lang> {
        match name.to_lowercase().as_str() {
            "java" => Some(Lang::Java),
            "python" | "py" => Some(Lang::Python),
            "javascript" | "js" => Some(Lang::JavaScript),
            "typescript" | "ts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::Tsx),
            "rust" | "rs" => Some(Lang::Rust),
            "go" => Some(Lang::Go),
            "ruby" | "rb" => Some(Lang::Ruby),
            "c" => Some(Lang::C),
            "cpp" | "cxx" => Some(Lang::Cpp),
            "csharp" | "cs" => Some(Lang::CSharp),
            "php" => Some(Lang::Php),
            "bash" | "sh" => Some(Lang::Bash),
            _ => None,
        }
    }

    pub fn grammar(&self) -> Language {
        match self {
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::Go => tree_sitter_go::LANGUAGE.into(),
            Lang::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Lang::C => tree_sitter_c::LANGUAGE.into(),
            Lang::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Lang::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Lang::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Lang::Bash => tree_sitter_bash::LANGUAGE.into(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Lang::Java => "java",
            Lang::Python => "python",
            Lang::JavaScript => "javascript",
            Lang::TypeScript => "typescript",
            Lang::Tsx => "tsx",
            Lang::Rust => "rust",
            Lang::Go => "go",
            Lang::Ruby => "ruby",
            Lang::C => "c",
            Lang::Cpp => "cpp",
            Lang::CSharp => "csharp",
            Lang::Php => "php",
            Lang::Bash => "bash",
        }
    }

    pub fn all() -> &'static [Lang] {
        &[
            Lang::Java, Lang::Python, Lang::JavaScript, Lang::TypeScript, Lang::Tsx,
            Lang::Rust, Lang::Go, Lang::Ruby, Lang::C, Lang::Cpp, Lang::CSharp,
            Lang::Php, Lang::Bash,
        ]
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
        assert!(matches!(Lang::from_extension("foo.tsx"), Some(Lang::Tsx)));
        assert!(matches!(Lang::from_extension("foo.rs"), Some(Lang::Rust)));
        assert!(matches!(Lang::from_extension("foo.go"), Some(Lang::Go)));
        assert!(matches!(Lang::from_extension("foo.rb"), Some(Lang::Ruby)));
        assert!(matches!(Lang::from_extension("foo.c"), Some(Lang::C)));
        assert!(matches!(Lang::from_extension("foo.h"), Some(Lang::C)));
        assert!(matches!(Lang::from_extension("foo.cpp"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_extension("foo.cc"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_extension("foo.hpp"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_extension("foo.cs"), Some(Lang::CSharp)));
        assert!(matches!(Lang::from_extension("foo.php"), Some(Lang::Php)));
        assert!(matches!(Lang::from_extension("foo.sh"), Some(Lang::Bash)));
        assert!(Lang::from_extension("foo.rs").is_some()); // no longer None
    }

    #[test]
    fn test_name_mapping() {
        assert!(matches!(Lang::from_name("java"), Some(Lang::Java)));
        assert!(matches!(Lang::from_name("python"), Some(Lang::Python)));
        assert!(matches!(Lang::from_name("py"), Some(Lang::Python)));
        assert!(matches!(Lang::from_name("js"), Some(Lang::JavaScript)));
        assert!(matches!(Lang::from_name("ts"), Some(Lang::TypeScript)));
        assert!(matches!(Lang::from_name("rs"), Some(Lang::Rust)));
        assert!(matches!(Lang::from_name("go"), Some(Lang::Go)));
        assert!(matches!(Lang::from_name("rb"), Some(Lang::Ruby)));
        assert!(matches!(Lang::from_name("c"), Some(Lang::C)));
        assert!(matches!(Lang::from_name("cpp"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_name("cs"), Some(Lang::CSharp)));
        assert!(matches!(Lang::from_name("php"), Some(Lang::Php)));
        assert!(matches!(Lang::from_name("bash"), Some(Lang::Bash)));
        assert!(Lang::from_name("ruby").is_some()); // no longer None
        assert!(Lang::from_name("kotlin").is_none());
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
        assert!(matches!(Lang::from_extension("test.hpp"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_extension("test.hh"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_extension("test.cxx"), Some(Lang::Cpp)));
        assert!(matches!(Lang::from_extension("test.zsh"), Some(Lang::Bash)));
    }

    #[test]
    fn test_grammar_loads() {
        for lang in &[Lang::Java, Lang::Python, Lang::JavaScript, Lang::Rust, Lang::Go, Lang::Ruby, Lang::C, Lang::Cpp, Lang::CSharp, Lang::Php, Lang::Bash] {
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

    #[test]
    fn test_all_includes_all() {
        let names: Vec<&str> = Lang::all().iter().map(|l| l.name()).collect();
        assert!(names.contains(&"rust"));
        assert!(names.contains(&"go"));
        assert_eq!(Lang::all().len(), 13);
    }
}
