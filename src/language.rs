//! Programming language detection and classification
//!
//! This module provides a centralized Language enum and extension-to-language
//! mapping to avoid duplication across comment, type, and import extraction modules.

use std::path::Path;

/// Supported programming languages for code analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    CSharp,
    Java,
    Kotlin,
    Swift,
    Ruby,
    PHP,
    Shell,
}

impl Language {
    /// Detect language from a file extension.
    ///
    /// Returns `None` if the extension is not recognized or not supported
    /// for code analysis.
    ///
    /// # Examples
    ///
    /// ```
    /// use fruit::language::Language;
    ///
    /// assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    /// assert_eq!(Language::from_extension("py"), Some(Language::Python));
    /// assert_eq!(Language::from_extension("jsx"), Some(Language::JavaScript));
    /// assert_eq!(Language::from_extension("unknown"), None);
    /// ```
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" | "pyw" | "pyi" => Some(Language::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
            "go" => Some(Language::Go),
            "c" | "h" => Some(Language::C),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" => Some(Language::Cpp),
            "cs" => Some(Language::CSharp),
            "java" => Some(Language::Java),
            "kt" | "kts" => Some(Language::Kotlin),
            "swift" => Some(Language::Swift),
            "rb" => Some(Language::Ruby),
            "php" => Some(Language::PHP),
            "sh" | "bash" | "zsh" | "fish" => Some(Language::Shell),
            _ => None,
        }
    }

    /// Detect language from a file path.
    ///
    /// Extracts the extension and calls `from_extension()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use fruit::language::Language;
    ///
    /// assert_eq!(Language::from_path(Path::new("main.rs")), Some(Language::Rust));
    /// assert_eq!(Language::from_path(Path::new("script.py")), Some(Language::Python));
    /// assert_eq!(Language::from_path(Path::new("README.md")), None);
    /// ```
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        Self::from_extension(ext)
    }

    /// Returns the canonical file extension for this language.
    ///
    /// This is useful for normalization and testing purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use fruit::language::Language;
    ///
    /// assert_eq!(Language::Rust.canonical_extension(), "rs");
    /// assert_eq!(Language::Python.canonical_extension(), "py");
    /// assert_eq!(Language::JavaScript.canonical_extension(), "js");
    /// ```
    pub fn canonical_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Go => "go",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::CSharp => "cs",
            Language::Java => "java",
            Language::Kotlin => "kt",
            Language::Swift => "swift",
            Language::Ruby => "rb",
            Language::PHP => "php",
            Language::Shell => "sh",
        }
    }

    /// Returns the human-readable name of the language.
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Go => "Go",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::CSharp => "C#",
            Language::Java => "Java",
            Language::Kotlin => "Kotlin",
            Language::Swift => "Swift",
            Language::Ruby => "Ruby",
            Language::PHP => "PHP",
            Language::Shell => "Shell",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_extension_basic() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
    }

    #[test]
    fn test_from_extension_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Some(Language::Rust));
        assert_eq!(Language::from_extension("Py"), Some(Language::Python));
        assert_eq!(Language::from_extension("JS"), Some(Language::JavaScript));
    }

    #[test]
    fn test_from_extension_variants() {
        // JavaScript variants
        assert_eq!(Language::from_extension("jsx"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("mjs"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("cjs"), Some(Language::JavaScript));

        // TypeScript variants
        assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("mts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("cts"), Some(Language::TypeScript));

        // C++ variants
        assert_eq!(Language::from_extension("cpp"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("cxx"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("hpp"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("hxx"), Some(Language::Cpp));

        // Kotlin variants
        assert_eq!(Language::from_extension("kt"), Some(Language::Kotlin));
        assert_eq!(Language::from_extension("kts"), Some(Language::Kotlin));

        // Shell variants
        assert_eq!(Language::from_extension("sh"), Some(Language::Shell));
        assert_eq!(Language::from_extension("bash"), Some(Language::Shell));
        assert_eq!(Language::from_extension("zsh"), Some(Language::Shell));
    }

    #[test]
    fn test_from_extension_unknown() {
        assert_eq!(Language::from_extension("unknown"), None);
        assert_eq!(Language::from_extension("txt"), None);
        assert_eq!(Language::from_extension("md"), None);
    }

    #[test]
    fn test_from_path() {
        assert_eq!(
            Language::from_path(Path::new("main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("script.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(Path::new("app.tsx")),
            Some(Language::TypeScript)
        );
        assert_eq!(Language::from_path(Path::new("README.md")), None);
        assert_eq!(Language::from_path(Path::new("Makefile")), None);
    }

    #[test]
    fn test_canonical_extension() {
        assert_eq!(Language::Rust.canonical_extension(), "rs");
        assert_eq!(Language::Python.canonical_extension(), "py");
        assert_eq!(Language::JavaScript.canonical_extension(), "js");
        assert_eq!(Language::TypeScript.canonical_extension(), "ts");
    }

    #[test]
    fn test_name() {
        assert_eq!(Language::Rust.name(), "Rust");
        assert_eq!(Language::Python.name(), "Python");
        assert_eq!(Language::JavaScript.name(), "JavaScript");
        assert_eq!(Language::Cpp.name(), "C++");
        assert_eq!(Language::CSharp.name(), "C#");
    }
}
