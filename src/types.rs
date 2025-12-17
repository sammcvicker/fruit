//! Lightweight type signature extraction using regex patterns
//!
//! Extracts exported type signatures and public APIs from source files.
//! This is a simpler approach than full tree-sitter integration, providing
//! ~80% of the value with much less complexity.

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use crate::metadata::{LineStyle, MetadataBlock, MetadataExtractor, MetadataLine};

/// Maximum file size for type extraction (1MB)
const MAX_FILE_SIZE: u64 = 1_000_000;

/// Extract exported type signatures from a file.
pub fn extract_type_signatures(path: &Path) -> Option<Vec<String>> {
    // Skip files that are too large
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > MAX_FILE_SIZE {
            return None;
        }
    }

    let extension = path.extension()?.to_str()?;
    let content = std::fs::read_to_string(path).ok()?;

    let signatures = match extension {
        "rs" => extract_rust_signatures(&content),
        "ts" | "tsx" | "mts" | "cts" => extract_typescript_signatures(&content),
        "js" | "jsx" | "mjs" | "cjs" => extract_javascript_signatures(&content),
        "py" => extract_python_signatures(&content),
        "go" => extract_go_signatures(&content),
        _ => None,
    };

    signatures.filter(|s| !s.is_empty())
}

// Static regex patterns for each language

// Rust patterns
static RUST_PUB_FN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pub\s+(async\s+)?fn\s+\w+[^{;]*").unwrap());
static RUST_PUB_STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pub\s+struct\s+\w+[^{;]*").unwrap());
static RUST_PUB_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pub\s+enum\s+\w+[^{;]*").unwrap());
static RUST_PUB_TRAIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pub\s+trait\s+\w+[^{;]*").unwrap());
static RUST_PUB_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pub\s+type\s+\w+[^;]+").unwrap());
static RUST_PUB_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pub\s+const\s+\w+:\s*[^=]+").unwrap());

fn extract_rust_signatures(content: &str) -> Option<Vec<String>> {
    let mut signatures = Vec::new();

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

        // Check each pattern
        if let Some(m) = RUST_PUB_FN.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = RUST_PUB_STRUCT.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = RUST_PUB_ENUM.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = RUST_PUB_TRAIT.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = RUST_PUB_TYPE.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = RUST_PUB_CONST.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        }
    }

    Some(signatures)
}

// TypeScript patterns
static TS_EXPORT_FUNCTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+(async\s+)?function\s+\w+[^{]*").unwrap());
static TS_EXPORT_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+interface\s+\w+[^{]*").unwrap());
static TS_EXPORT_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+type\s+\w+[^=]*=\s*[^;{]+").unwrap());
static TS_EXPORT_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+(abstract\s+)?class\s+\w+[^{]*").unwrap());
static TS_EXPORT_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+const\s+\w+:\s*[^=]+").unwrap());
static TS_EXPORT_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+(const\s+)?enum\s+\w+[^{]*").unwrap());

fn extract_typescript_signatures(content: &str) -> Option<Vec<String>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        // Check each pattern
        if let Some(m) = TS_EXPORT_FUNCTION.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = TS_EXPORT_INTERFACE.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = TS_EXPORT_TYPE.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = TS_EXPORT_CLASS.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = TS_EXPORT_CONST.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = TS_EXPORT_ENUM.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        }
    }

    Some(signatures)
}

// JavaScript patterns (subset of TypeScript, no type annotations)
static JS_EXPORT_FUNCTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+(async\s+)?function\s+\w+\s*\([^)]*\)").unwrap());
static JS_EXPORT_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+class\s+\w+[^{]*").unwrap());
static JS_EXPORT_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export\s+const\s+\w+\s*=").unwrap());

fn extract_javascript_signatures(content: &str) -> Option<Vec<String>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        // Check each pattern
        if let Some(m) = JS_EXPORT_FUNCTION.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = JS_EXPORT_CLASS.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = JS_EXPORT_CONST.find(trimmed) {
            // For const, just show the declaration without the value
            let sig = m.as_str().trim_end_matches('=').trim();
            signatures.push(sig.to_string());
        }
    }

    Some(signatures)
}

// Python patterns
static PY_DEF_WITH_RETURN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^def\s+\w+\s*\([^)]*\)\s*->\s*[^:]+").unwrap());
static PY_ASYNC_DEF_WITH_RETURN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^async\s+def\s+\w+\s*\([^)]*\)\s*->\s*[^:]+").unwrap());
static PY_CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^class\s+\w+[^:]*").unwrap());

fn extract_python_signatures(content: &str) -> Option<Vec<String>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Skip private functions/classes (starting with _)
        if trimmed.starts_with("def _")
            || trimmed.starts_with("async def _")
            || trimmed.starts_with("class _")
        {
            continue;
        }

        // Check each pattern (async first to avoid partial matches)
        if let Some(m) = PY_ASYNC_DEF_WITH_RETURN.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = PY_DEF_WITH_RETURN.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = PY_CLASS.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        }
    }

    Some(signatures)
}

// Go patterns - exported items start with uppercase
static GO_EXPORTED_FUNC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^func\s+[A-Z]\w*\s*\([^)]*\)[^{]*").unwrap());
static GO_EXPORTED_METHOD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^func\s+\([^)]+\)\s*[A-Z]\w*\s*\([^)]*\)[^{]*").unwrap());
static GO_EXPORTED_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^type\s+[A-Z]\w*\s+\w+").unwrap());
static GO_EXPORTED_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^const\s+[A-Z]\w*\s*[^=]*=").unwrap());
static GO_EXPORTED_VAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^var\s+[A-Z]\w*\s+\w+").unwrap());

fn extract_go_signatures(content: &str) -> Option<Vec<String>> {
    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        // Check each pattern (method before func to get receiver)
        if let Some(m) = GO_EXPORTED_METHOD.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = GO_EXPORTED_FUNC.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = GO_EXPORTED_TYPE.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
        } else if let Some(m) = GO_EXPORTED_CONST.find(trimmed) {
            let sig = m.as_str().trim_end_matches('=').trim();
            signatures.push(sig.to_string());
        } else if let Some(m) = GO_EXPORTED_VAR.find(trimmed) {
            signatures.push(clean_signature(m.as_str()));
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
        extract_type_signatures(path).map(|signatures| {
            let lines = signatures
                .into_iter()
                .map(|sig| MetadataLine::with_style(sig, LineStyle::TypeSignature))
                .collect();
            MetadataBlock::new("types", lines)
        })
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
        assert!(sigs[0].starts_with("pub fn process"));
        assert!(sigs[1].starts_with("pub async fn async_process"));
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
        assert!(sigs[0].starts_with("pub struct Config"));
        assert!(sigs[1].starts_with("pub struct Generic"));
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
        assert!(sigs[0].starts_with("pub trait Handler"));
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
        assert!(sigs[0].starts_with("pub enum Status"));
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
        assert!(sigs[0].starts_with("export interface User"));
        assert!(sigs[1].starts_with("export type UserId"));
        assert!(sigs[2].starts_with("export function getUser"));
        assert!(sigs[3].starts_with("export async function createUser"));
        assert!(sigs[4].starts_with("export const API_URL"));
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
        assert!(sigs[0].starts_with("export class UserService"));
        assert!(sigs[1].starts_with("export abstract class BaseHandler"));
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
        assert!(sigs[0].starts_with("export function calculate"));
        assert!(sigs[1].starts_with("export async function fetchData"));
        assert!(sigs[2].starts_with("export class Calculator"));
        assert!(sigs[3].starts_with("export const VERSION"));
    }

    #[test]
    fn test_python_typed_functions() {
        let content = r#"
"""Module docstring."""

def process(data: str) -> dict:
    """Process data."""
    return {}

async def fetch(url: str) -> bytes:
    """Fetch from URL."""
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
        assert_eq!(sigs.len(), 3);
        assert!(sigs[0].starts_with("def process"));
        assert!(sigs[1].starts_with("async def fetch"));
        assert!(sigs[2].starts_with("class UserService"));
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
        assert!(sigs[0].starts_with("type Config struct"));
        assert!(sigs[1].starts_with("func NewConfig()"));
        assert!(sigs[2].starts_with("func (c *Config) Validate()"));
        assert!(sigs[3].starts_with("const DefaultPort"));
        assert!(sigs[4].starts_with("var GlobalConfig"));
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
}
