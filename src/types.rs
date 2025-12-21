//! Lightweight type signature extraction using regex patterns
//!
//! Extracts exported type signatures and public APIs from source files.
//! This is a simpler approach than full tree-sitter integration, providing
//! ~80% of the value with much less complexity.

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use crate::file_utils::read_source_file;
use crate::language::Language;
use crate::metadata::{MetadataBlock, MetadataExtractor};

/// A type signature extracted from source code.
///
/// Represents a public API element (function, class, struct, etc.) with its
/// signature text, symbol name, and indentation level for display hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSignature {
    /// The full signature text (e.g., "pub fn process(input: &str) -> Result<String, Error>")
    pub signature: String,
    /// The symbol name for highlighting (e.g., "process")
    pub symbol_name: String,
    /// Indentation level in spaces (tabs converted to 4 spaces)
    pub indent: usize,
}

impl TypeSignature {
    /// Create a new type signature.
    pub fn new(signature: String, symbol_name: String, indent: usize) -> Self {
        Self {
            signature,
            symbol_name,
            indent,
        }
    }
}

/// Calculate the indentation level of a line (number of spaces, tabs = 4 spaces).
fn calculate_indent(line: &str) -> usize {
    let mut indent = 0;
    for ch in line.chars() {
        match ch {
            ' ' => indent += 1,
            '\t' => indent += 4,
            _ => break,
        }
    }
    indent
}

/// Extract exported type signatures from a file.
///
/// Returns a list of type signatures with their symbol names and indentation levels.
/// Indentation is measured in spaces (tabs are converted to 4 spaces).
pub fn extract_type_signatures(path: &Path) -> Option<Vec<TypeSignature>> {
    let (content, _extension) = read_source_file(path)?;
    let language = Language::from_path(path)?;

    let signatures = match language {
        Language::Rust => extract_rust_signatures(&content),
        Language::TypeScript => extract_typescript_signatures(&content),
        Language::JavaScript => extract_javascript_signatures(&content),
        Language::Python => extract_python_signatures(&content),
        Language::Go => extract_go_signatures(&content),
        _ => None,
    };

    signatures.filter(|s| !s.is_empty())
}

// Static regex patterns for each language

// Rust patterns - with capture groups for symbol names
static RUST_PUB_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^pub\s+(async\s+)?fn\s+(\w+)[^{;]*").expect("RUST_PUB_FN regex is invalid")
});
static RUST_PUB_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^pub\s+struct\s+(\w+)[^{;]*").expect("RUST_PUB_STRUCT regex is invalid")
});
static RUST_PUB_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^pub\s+enum\s+(\w+)[^{;]*").expect("RUST_PUB_ENUM regex is invalid")
});
static RUST_PUB_TRAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^pub\s+trait\s+(\w+)[^{;]*").expect("RUST_PUB_TRAIT regex is invalid")
});
static RUST_PUB_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^pub\s+type\s+(\w+)[^;]+").expect("RUST_PUB_TYPE regex is invalid")
});
static RUST_PUB_CONST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^pub\s+const\s+(\w+):\s*[^=]+").expect("RUST_PUB_CONST regex is invalid")
});
static RUST_IMPL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^impl(?:\s*<[^>]+>)?\s+(\w+)").expect("RUST_IMPL regex is invalid")
});
static RUST_IMPL_TRAIT_FOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^impl(?:\s*<[^>]+>)?\s+(\w+)\s+for\s+(\w+)")
        .expect("RUST_IMPL_TRAIT_FOR regex is invalid")
});

fn extract_rust_signatures(content: &str) -> Option<Vec<TypeSignature>> {
    let mut signatures = Vec::new();
    let mut in_impl_block = false;
    let mut impl_indent: usize = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip doc comments and attributes
        if trimmed.starts_with("///")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("#[")
            || trimmed.starts_with("//")
        {
            continue;
        }

        let indent = calculate_indent(line);

        // Check for end of impl block (closing brace at same or lower indent)
        if in_impl_block && trimmed == "}" && indent <= impl_indent {
            in_impl_block = false;
        }

        // Check for impl blocks first (impl Trait for Type or impl Type)
        if let Some(caps) = RUST_IMPL_TRAIT_FOR.captures(trimmed) {
            // impl Trait for Type
            if let (Some(full), Some(trait_match), Some(type_match)) =
                (caps.get(0), caps.get(1), caps.get(2))
            {
                let sig = clean_signature(full.as_str());
                let symbol = format!("{} for {}", trait_match.as_str(), type_match.as_str());
                signatures.push(TypeSignature::new(sig, symbol, indent));
                in_impl_block = true;
                impl_indent = indent;
            }
        } else if let Some(caps) = RUST_IMPL.captures(trimmed) {
            // impl Type
            if let (Some(full), Some(type_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, type_match.as_str().to_string(), indent));
                in_impl_block = true;
                impl_indent = indent;
            }
        } else if in_impl_block {
            // Inside impl block, capture pub fn and fn (both public and private associated functions)
            if let Some(caps) = Regex::new(r"^(pub\s+)?(async\s+)?fn\s+(\w+)[^{;]*")
                .unwrap()
                .captures(trimmed)
            {
                if let (Some(full), Some(fn_name)) = (caps.get(0), caps.get(3)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, fn_name.as_str().to_string(), indent));
                }
            }
        } else {
            // Top-level declarations (not inside impl block)
            // Check each pattern - capture group index varies for fn (has optional async)
            // Use pattern matching to safely handle capture groups
            if let Some(caps) = RUST_PUB_FN.captures(trimmed) {
                if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(2)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                }
            } else if let Some(caps) = RUST_PUB_STRUCT.captures(trimmed) {
                if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                }
            } else if let Some(caps) = RUST_PUB_ENUM.captures(trimmed) {
                if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                }
            } else if let Some(caps) = RUST_PUB_TRAIT.captures(trimmed) {
                if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                }
            } else if let Some(caps) = RUST_PUB_TYPE.captures(trimmed) {
                if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                }
            } else if let Some(caps) = RUST_PUB_CONST.captures(trimmed) {
                if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                    let sig = clean_signature(full.as_str());
                    signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                }
            }
        }
    }

    Some(signatures)
}

// TypeScript patterns - with capture groups for symbol names
static TS_EXPORT_FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+(async\s+)?function\s+(\w+)[^{]*")
        .expect("TS_EXPORT_FUNCTION regex is invalid")
});
static TS_EXPORT_INTERFACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+interface\s+(\w+)[^{]*").expect("TS_EXPORT_INTERFACE regex is invalid")
});
static TS_EXPORT_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+type\s+(\w+)[^=]*=\s*[^;{]+").expect("TS_EXPORT_TYPE regex is invalid")
});
static TS_EXPORT_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+(abstract\s+)?class\s+(\w+)[^{]*")
        .expect("TS_EXPORT_CLASS regex is invalid")
});
static TS_EXPORT_CONST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+const\s+(\w+):\s*[^=]+").expect("TS_EXPORT_CONST regex is invalid")
});
static TS_EXPORT_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+(const\s+)?enum\s+(\w+)[^{]*").expect("TS_EXPORT_ENUM regex is invalid")
});

fn extract_typescript_signatures(content: &str) -> Option<Vec<TypeSignature>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        let indent = calculate_indent(line);

        // Check each pattern - capture group index varies for function/class/enum (have optional modifiers)
        // Use pattern matching to safely handle capture groups
        if let Some(caps) = TS_EXPORT_FUNCTION.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(2)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = TS_EXPORT_INTERFACE.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = TS_EXPORT_TYPE.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = TS_EXPORT_CLASS.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(2)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = TS_EXPORT_CONST.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = TS_EXPORT_ENUM.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(2)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        }
    }

    Some(signatures)
}

// JavaScript patterns (subset of TypeScript, no type annotations) - with capture groups
static JS_EXPORT_FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+(async\s+)?function\s+(\w+)\s*\([^)]*\)")
        .expect("JS_EXPORT_FUNCTION regex is invalid")
});
static JS_EXPORT_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+class\s+(\w+)[^{]*").expect("JS_EXPORT_CLASS regex is invalid")
});
static JS_EXPORT_CONST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^export\s+const\s+(\w+)\s*=").expect("JS_EXPORT_CONST regex is invalid")
});

fn extract_javascript_signatures(content: &str) -> Option<Vec<TypeSignature>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        let indent = calculate_indent(line);

        // Check each pattern - use pattern matching to safely handle capture groups
        if let Some(caps) = JS_EXPORT_FUNCTION.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(2)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = JS_EXPORT_CLASS.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = JS_EXPORT_CONST.captures(trimmed) {
            // For const, just show the declaration without the value
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = full.as_str().trim_end_matches('=').trim();
                signatures.push(TypeSignature::new(sig.to_string(), sym_match.as_str().to_string(), indent));
            }
        }
    }

    Some(signatures)
}

// Python patterns - with capture groups for symbol names
// Functions with return type annotations (preferred, more informative)
static PY_DEF_WITH_RETURN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^def\s+(\w+)\s*\([^)]*\)\s*->\s*[^:]+")
        .expect("PY_DEF_WITH_RETURN regex is invalid")
});
static PY_ASYNC_DEF_WITH_RETURN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^async\s+def\s+(\w+)\s*\([^)]*\)\s*->\s*[^:]+")
        .expect("PY_ASYNC_DEF_WITH_RETURN regex is invalid")
});
// Functions without return type annotations (fallback)
static PY_DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^def\s+(\w+)\s*\([^)]*\)").expect("PY_DEF regex is invalid"));
static PY_ASYNC_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^async\s+def\s+(\w+)\s*\([^)]*\)").expect("PY_ASYNC_DEF regex is invalid")
});
static PY_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^class\s+(\w+)[^:]*").expect("PY_CLASS regex is invalid"));

fn extract_python_signatures(content: &str) -> Option<Vec<TypeSignature>> {
    let mut signatures = Vec::new();
    let mut pending_decorators: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Track decorators for the next function/class
        if trimmed.starts_with('@') {
            pending_decorators.push(trimmed.to_string());
            continue;
        }

        // Skip private functions/classes (starting with _)
        if trimmed.starts_with("def _")
            || trimmed.starts_with("async def _")
            || trimmed.starts_with("class _")
        {
            pending_decorators.clear(); // Clear decorators for private items
            continue;
        }

        let indent = calculate_indent(line);

        // Check each pattern (async first to avoid partial matches)
        // Try typed versions first (more informative), then fall back to untyped
        // Use pattern matching to safely handle capture groups
        let mut matched = false;

        if let Some(caps) = PY_ASYNC_DEF_WITH_RETURN.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = build_signature_with_decorators(&pending_decorators, full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                matched = true;
            }
        } else if let Some(caps) = PY_ASYNC_DEF.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = build_signature_with_decorators(&pending_decorators, full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                matched = true;
            }
        } else if let Some(caps) = PY_DEF_WITH_RETURN.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = build_signature_with_decorators(&pending_decorators, full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                matched = true;
            }
        } else if let Some(caps) = PY_DEF.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = build_signature_with_decorators(&pending_decorators, full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                matched = true;
            }
        } else if let Some(caps) = PY_CLASS.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = build_signature_with_decorators(&pending_decorators, full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
                matched = true;
            }
        }

        // Clear pending decorators after matching or if we hit a non-decorator, non-def line
        if matched
            || (!trimmed.is_empty()
                && !trimmed.starts_with("def")
                && !trimmed.starts_with("async def")
                && !trimmed.starts_with("class"))
        {
            pending_decorators.clear();
        }
    }

    Some(signatures)
}

/// Build a signature string including decorators if present
fn build_signature_with_decorators(decorators: &[String], def_line: &str) -> String {
    if decorators.is_empty() {
        clean_signature(def_line)
    } else {
        let decorator_str = decorators.join(" ");
        format!("{} {}", decorator_str, clean_signature(def_line))
    }
}

// Go patterns - exported items start with uppercase - with capture groups
static GO_EXPORTED_FUNC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^func\s+([A-Z]\w*)\s*\([^)]*\)[^{]*").expect("GO_EXPORTED_FUNC regex is invalid")
});
static GO_EXPORTED_METHOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^func\s+\([^)]+\)\s*([A-Z]\w*)\s*\([^)]*\)[^{]*")
        .expect("GO_EXPORTED_METHOD regex is invalid")
});
static GO_EXPORTED_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^type\s+([A-Z]\w*)\s+\w+").expect("GO_EXPORTED_TYPE regex is invalid")
});
static GO_EXPORTED_CONST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^const\s+([A-Z]\w*)\s*[^=]*=").expect("GO_EXPORTED_CONST regex is invalid")
});
static GO_EXPORTED_VAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^var\s+([A-Z]\w*)\s+\w+").expect("GO_EXPORTED_VAR regex is invalid")
});

fn extract_go_signatures(content: &str) -> Option<Vec<TypeSignature>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        let indent = calculate_indent(line);

        // Check each pattern (method before func to get receiver)
        // Use pattern matching to safely handle capture groups
        if let Some(caps) = GO_EXPORTED_METHOD.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = GO_EXPORTED_FUNC.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = GO_EXPORTED_TYPE.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = GO_EXPORTED_CONST.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = full.as_str().trim_end_matches('=').trim();
                signatures.push(TypeSignature::new(sig.to_string(), sym_match.as_str().to_string(), indent));
            }
        } else if let Some(caps) = GO_EXPORTED_VAR.captures(trimmed) {
            if let (Some(full), Some(sym_match)) = (caps.get(0), caps.get(1)) {
                let sig = clean_signature(full.as_str());
                signatures.push(TypeSignature::new(sig, sym_match.as_str().to_string(), indent));
            }
        }
    }

    Some(signatures)
}

/// Clean up a signature by trimming whitespace and removing trailing braces/semicolons
fn clean_signature(sig: &str) -> String {
    sig.trim()
        .trim_end_matches('{')
        .trim_end_matches(';')
        .trim()
        .to_string()
}

/// Type signature extractor that implements the MetadataExtractor trait.
pub struct TypeExtractor;

impl MetadataExtractor for TypeExtractor {
    fn extract(&self, path: &Path) -> Option<MetadataBlock> {
        extract_type_signatures(path).map(MetadataBlock::from_types)
    }

    fn name(&self) -> &'static str {
        "types"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_pub_fn() {
        let content = r#"
/// Doc comment
pub fn process(input: &str) -> Result<String, Error> {
    todo!()
}

fn private_fn() {}

pub async fn async_process(data: Vec<u8>) -> io::Result<()> {
    todo!()
}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 2);
        assert!(sigs[0].signature.starts_with("pub fn process"));
        assert_eq!(sigs[0].symbol_name, "process");
        assert!(sigs[1].signature.starts_with("pub async fn async_process"));
        assert_eq!(sigs[1].symbol_name, "async_process");
    }

    #[test]
    fn test_rust_pub_struct() {
        let content = r#"
pub struct Config {
    pub host: String,
    pub port: u16,
}

struct PrivateStruct {}

pub struct Generic<T: Clone> {
    data: T,
}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 2);
        assert!(sigs[0].signature.starts_with("pub struct Config"));
        assert_eq!(sigs[0].symbol_name, "Config");
        assert!(sigs[1].signature.starts_with("pub struct Generic"));
        assert_eq!(sigs[1].symbol_name, "Generic");
    }

    #[test]
    fn test_rust_pub_trait() {
        let content = r#"
pub trait Handler {
    fn handle(&self, req: Request) -> Response;
}

trait Private {}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 1);
        assert!(sigs[0].signature.starts_with("pub trait Handler"));
        assert_eq!(sigs[0].symbol_name, "Handler");
    }

    #[test]
    fn test_rust_pub_enum() {
        let content = r#"
pub enum Status {
    Active,
    Inactive,
}

enum Private {}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 1);
        assert!(sigs[0].signature.starts_with("pub enum Status"));
        assert_eq!(sigs[0].symbol_name, "Status");
    }

    #[test]
    fn test_typescript_exports() {
        let content = r#"
// File comment
export interface User {
    id: string;
    name: string;
}

export type UserId = string;

export function getUser(id: string): Promise<User> {
    return fetch(`/users/${id}`);
}

export async function createUser(data: UserInput): Promise<User> {
    return post('/users', data);
}

export const API_URL: string = "https://api.example.com";

function privateFunc() {}
"#;
        let sigs = extract_typescript_signatures(content).unwrap();
        assert_eq!(sigs.len(), 5);
        assert!(sigs[0].signature.starts_with("export interface User"));
        assert_eq!(sigs[0].symbol_name, "User");
        assert!(sigs[1].signature.starts_with("export type UserId"));
        assert_eq!(sigs[1].symbol_name, "UserId");
        assert!(sigs[2].signature.starts_with("export function getUser"));
        assert_eq!(sigs[2].symbol_name, "getUser");
        assert!(sigs[3].signature.starts_with("export async function createUser"));
        assert_eq!(sigs[3].symbol_name, "createUser");
        assert!(sigs[4].signature.starts_with("export const API_URL"));
        assert_eq!(sigs[4].symbol_name, "API_URL");
    }

    #[test]
    fn test_typescript_class() {
        let content = r#"
export class UserService {
    constructor(private api: ApiClient) {}
}

export abstract class BaseHandler {
    abstract handle(): void;
}
"#;
        let sigs = extract_typescript_signatures(content).unwrap();
        assert_eq!(sigs.len(), 2);
        assert!(sigs[0].signature.starts_with("export class UserService"));
        assert_eq!(sigs[0].symbol_name, "UserService");
        assert!(sigs[1].signature.starts_with("export abstract class BaseHandler"));
        assert_eq!(sigs[1].symbol_name, "BaseHandler");
    }

    #[test]
    fn test_javascript_exports() {
        let content = r#"
export function calculate(a, b) {
    return a + b;
}

export async function fetchData(url) {
    return fetch(url);
}

export class Calculator {
    add(a, b) { return a + b; }
}

export const VERSION = "1.0.0";

function privateFunc() {}
"#;
        let sigs = extract_javascript_signatures(content).unwrap();
        assert_eq!(sigs.len(), 4);
        assert!(sigs[0].signature.starts_with("export function calculate"));
        assert_eq!(sigs[0].symbol_name, "calculate");
        assert!(sigs[1].signature.starts_with("export async function fetchData"));
        assert_eq!(sigs[1].symbol_name, "fetchData");
        assert!(sigs[2].signature.starts_with("export class Calculator"));
        assert_eq!(sigs[2].symbol_name, "Calculator");
        assert!(sigs[3].signature.starts_with("export const VERSION"));
        assert_eq!(sigs[3].symbol_name, "VERSION");
    }

    #[test]
    fn test_python_functions() {
        let content = r#"
"""Module docstring."""

def process(data: str) -> dict:
    """Process data."""
    return {}

def simple_func(items):
    """Simple function without type annotation."""
    return items

async def fetch(url: str) -> bytes:
    """Fetch from URL."""
    pass

async def fetch_untyped(url):
    """Async function without type annotation."""
    pass

class UserService:
    """User service class."""
    pass

def _private_func() -> None:
    pass

class _PrivateClass:
    pass
"#;
        let sigs = extract_python_signatures(content).unwrap();
        assert_eq!(sigs.len(), 5, "should capture 5 signatures: {:?}", sigs);

        // Typed functions
        assert!(sigs[0].signature.starts_with("def process"));
        assert_eq!(sigs[0].symbol_name, "process");

        // Untyped function
        assert!(sigs[1].signature.starts_with("def simple_func"));
        assert_eq!(sigs[1].symbol_name, "simple_func");

        // Typed async
        assert!(sigs[2].signature.starts_with("async def fetch"));
        assert_eq!(sigs[2].symbol_name, "fetch");

        // Untyped async
        assert!(sigs[3].signature.starts_with("async def fetch_untyped"));
        assert_eq!(sigs[3].symbol_name, "fetch_untyped");

        // Class
        assert!(sigs[4].signature.starts_with("class UserService"));
        assert_eq!(sigs[4].symbol_name, "UserService");
    }

    #[test]
    fn test_python_typed_functions() {
        // Legacy test for backward compatibility - typed functions still work
        let content = r#"
def process(data: str) -> dict:
    return {}

async def fetch(url: str) -> bytes:
    pass

class UserService:
    pass
"#;
        let sigs = extract_python_signatures(content).unwrap();
        assert_eq!(sigs.len(), 3);
        assert!(sigs[0].signature.contains("->"));
        assert!(sigs[1].signature.contains("->"));
    }

    #[test]
    fn test_python_decorated_functions() {
        let content = r#"
class MyClass:
    @property
    def name(self) -> str:
        return self._name

    @staticmethod
    def create() -> 'MyClass':
        return MyClass()

    @classmethod
    def from_dict(cls, data: dict) -> 'MyClass':
        return cls()

    @app.route('/users')
    def get_users() -> list:
        return []

    @decorator1
    @decorator2
    def multi_decorated(x: int) -> int:
        return x

    def regular_method(self):
        pass
"#;
        let sigs = extract_python_signatures(content).unwrap();
        assert_eq!(sigs.len(), 7); // class + 6 methods

        // Check class
        assert!(sigs[0].signature.starts_with("class MyClass"));
        assert_eq!(sigs[0].symbol_name, "MyClass");

        // Check property decorator
        assert!(sigs[1].signature.contains("@property"));
        assert!(sigs[1].signature.contains("def name(self) -> str"));
        assert_eq!(sigs[1].symbol_name, "name");

        // Check staticmethod decorator
        assert!(sigs[2].signature.contains("@staticmethod"));
        assert!(sigs[2].signature.contains("def create() -> 'MyClass'"));
        assert_eq!(sigs[2].symbol_name, "create");

        // Check classmethod decorator
        assert!(sigs[3].signature.contains("@classmethod"));
        assert!(
            sigs[3]
                .signature
                .contains("def from_dict(cls, data: dict) -> 'MyClass'")
        );
        assert_eq!(sigs[3].symbol_name, "from_dict");

        // Check custom decorator
        assert!(sigs[4].signature.contains("@app.route('/users')"));
        assert!(sigs[4].signature.contains("def get_users() -> list"));
        assert_eq!(sigs[4].symbol_name, "get_users");

        // Check multiple decorators
        assert!(sigs[5].signature.contains("@decorator1"));
        assert!(sigs[5].signature.contains("@decorator2"));
        assert!(sigs[5].signature.contains("def multi_decorated(x: int) -> int"));
        assert_eq!(sigs[5].symbol_name, "multi_decorated");

        // Check regular method (no decorator)
        assert!(!sigs[6].signature.contains("@"));
        assert!(sigs[6].signature.contains("def regular_method(self)"));
        assert_eq!(sigs[6].symbol_name, "regular_method");
    }

    #[test]
    fn test_python_private_decorated_functions() {
        let content = r#"
@property
def _private_property(self) -> str:
    return "private"

@staticmethod
def public_method() -> None:
    pass
"#;
        let sigs = extract_python_signatures(content).unwrap();
        // Should skip _private_property but capture public_method
        assert_eq!(sigs.len(), 1);
        assert!(sigs[0].signature.contains("@staticmethod"));
        assert_eq!(sigs[0].symbol_name, "public_method");
    }

    #[test]
    fn test_go_exports() {
        let content = r#"
// Package main is the entry point.
package main

// Config holds configuration.
type Config struct {
    Host string
    Port int
}

func NewConfig() *Config {
    return &Config{}
}

func (c *Config) Validate() error {
    return nil
}

const DefaultPort = 8080

var GlobalConfig Config

// private stuff
type privateType struct{}
func privateFunc() {}
"#;
        let sigs = extract_go_signatures(content).unwrap();
        assert_eq!(sigs.len(), 5);
        assert!(sigs[0].signature.starts_with("type Config struct"));
        assert_eq!(sigs[0].symbol_name, "Config");
        assert!(sigs[1].signature.starts_with("func NewConfig()"));
        assert_eq!(sigs[1].symbol_name, "NewConfig");
        assert!(sigs[2].signature.starts_with("func (c *Config) Validate()"));
        assert_eq!(sigs[2].symbol_name, "Validate");
        assert!(sigs[3].signature.starts_with("const DefaultPort"));
        assert_eq!(sigs[3].symbol_name, "DefaultPort");
        assert!(sigs[4].signature.starts_with("var GlobalConfig"));
        assert_eq!(sigs[4].symbol_name, "GlobalConfig");
    }

    #[test]
    fn test_clean_signature() {
        assert_eq!(clean_signature("pub fn foo() {"), "pub fn foo()");
        assert_eq!(
            clean_signature("export interface Foo {"),
            "export interface Foo"
        );
        assert_eq!(clean_signature("type Bar struct {"), "type Bar struct");
        assert_eq!(clean_signature("  spaced  "), "spaced");
    }

    #[test]
    fn test_empty_file() {
        let content = "";
        let sigs = extract_rust_signatures(content).unwrap();
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_no_exports() {
        let content = r#"
// Internal module
fn private_func() {}
struct Private {}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_rust_impl_blocks() {
        let content = r#"
pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn new() -> Self {
        Config {
            host: "localhost".to_string(),
            port: 8080,
        }
    }

    pub fn with_port(port: u16) -> Self {
        Config {
            host: "localhost".to_string(),
            port,
        }
    }

    fn private_method(&self) -> bool {
        true
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

pub async fn standalone_fn() -> Result<(), Error> {
    Ok(())
}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 8);

        // pub struct Config
        assert!(sigs[0].signature.starts_with("pub struct Config"));
        assert_eq!(sigs[0].symbol_name, "Config");

        // impl Config
        assert!(sigs[1].signature.starts_with("impl Config"));
        assert_eq!(sigs[1].symbol_name, "Config");

        // pub fn new() -> Self
        assert!(sigs[2].signature.starts_with("pub fn new() -> Self"));
        assert_eq!(sigs[2].symbol_name, "new");
        assert!(sigs[2].indent > sigs[1].indent); // Should be indented relative to impl

        // pub fn with_port(port: u16) -> Self
        assert!(sigs[3].signature.starts_with("pub fn with_port(port: u16) -> Self"));
        assert_eq!(sigs[3].symbol_name, "with_port");

        // fn private_method(&self) -> bool
        assert!(sigs[4].signature.starts_with("fn private_method(&self) -> bool"));
        assert_eq!(sigs[4].symbol_name, "private_method");

        // impl Default for Config
        assert!(sigs[5].signature.starts_with("impl Default for Config"));
        assert_eq!(sigs[5].symbol_name, "Default for Config");

        // fn default() -> Self
        assert!(sigs[6].signature.starts_with("fn default() -> Self"));
        assert_eq!(sigs[6].symbol_name, "default");

        // pub async fn standalone_fn() -> Result<(), Error>
        assert!(sigs[7].signature.starts_with("pub async fn standalone_fn"));
        assert_eq!(sigs[7].symbol_name, "standalone_fn");
    }

    #[test]
    fn test_rust_impl_with_generics() {
        let content = r#"
pub struct Container<T> {
    value: T,
}

impl<T: Clone> Container<T> {
    pub fn new(value: T) -> Self {
        Container { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }
}

impl<T: Default> Default for Container<T> {
    fn default() -> Self {
        Container {
            value: T::default(),
        }
    }
}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 6);

        // pub struct Container<T>
        assert!(sigs[0].signature.starts_with("pub struct Container"));
        assert_eq!(sigs[0].symbol_name, "Container");

        // impl<T: Clone> Container<T>
        assert!(sigs[1].signature.starts_with("impl"));
        assert_eq!(sigs[1].symbol_name, "Container");

        // pub fn new(value: T) -> Self
        assert!(sigs[2].signature.starts_with("pub fn new(value: T) -> Self"));
        assert_eq!(sigs[2].symbol_name, "new");

        // pub fn get(&self) -> &T
        assert!(sigs[3].signature.starts_with("pub fn get(&self) -> &T"));
        assert_eq!(sigs[3].symbol_name, "get");

        // impl<T: Default> Default for Container<T>
        assert!(sigs[4].signature.starts_with("impl"));
        assert_eq!(sigs[4].symbol_name, "Default for Container");

        // fn default() -> Self
        assert!(sigs[5].signature.starts_with("fn default() -> Self"));
        assert_eq!(sigs[5].symbol_name, "default");
    }

    #[test]
    fn test_rust_nested_impl_blocks() {
        // Test that we don't get confused by nested blocks
        let content = r#"
pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn new() -> Self {
        Outer {
            inner: Inner::new(),
        }
    }

    pub fn process(&self) {
        if true {
            // nested block
            let x = 5;
        }
    }
}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 4);

        assert!(sigs[0].signature.starts_with("pub struct Outer"));
        assert!(sigs[1].signature.starts_with("impl Outer"));
        assert!(sigs[2].signature.starts_with("pub fn new() -> Self"));
        assert!(sigs[3].signature.starts_with("pub fn process(&self)"));
    }

    #[test]
    fn test_rust_async_in_impl() {
        let content = r#"
pub struct AsyncService {}

impl AsyncService {
    pub async fn fetch(&self, url: &str) -> Result<String, Error> {
        Ok(String::new())
    }

    async fn internal_fetch(&self) -> String {
        String::new()
    }
}
"#;
        let sigs = extract_rust_signatures(content).unwrap();
        assert_eq!(sigs.len(), 4);

        assert!(sigs[0].signature.starts_with("pub struct AsyncService"));
        assert!(sigs[1].signature.starts_with("impl AsyncService"));
        assert!(sigs[2].signature.starts_with("pub async fn fetch"));
        assert_eq!(sigs[2].symbol_name, "fetch");
        assert!(sigs[3].signature.starts_with("async fn internal_fetch"));
        assert_eq!(sigs[3].symbol_name, "internal_fetch");
    }
}
