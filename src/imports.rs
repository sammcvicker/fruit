//! Import/dependency extraction from source files
//!
//! Extracts import statements from source files to provide a quick view
//! of file dependencies and what external modules each file relies on.

use regex::Regex;
use serde::Serialize;
use std::path::Path;
use std::sync::LazyLock;

use crate::file_utils::read_source_file;

/// Categorized imports from a source file.
#[derive(Debug, Clone, Default, Serialize)]
pub struct FileImports {
    /// External package/crate imports
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub external: Vec<String>,
    /// Standard library imports
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub std: Vec<String>,
    /// Internal/project imports
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub internal: Vec<String>,
}

impl FileImports {
    pub fn is_empty(&self) -> bool {
        self.external.is_empty() && self.std.is_empty() && self.internal.is_empty()
    }

    /// Get total number of imports
    pub fn total(&self) -> usize {
        self.external.len() + self.std.len() + self.internal.len()
    }

    /// Get a summary string for display
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.external.is_empty() {
            parts.push(self.external.join(", "));
        }
        if !self.std.is_empty() {
            parts.push(format!("std::{{{}}}", self.std.join(", ")));
        }
        if !self.internal.is_empty() {
            parts.push(format!("crate::{{{}}}", self.internal.join(", ")));
        }
        parts.join(", ")
    }
}

/// Extract imports from a file.
pub fn extract_imports(path: &Path) -> Option<FileImports> {
    let (content, extension) = read_source_file(path)?;

    let imports = match extension {
        "rs" => extract_rust_imports(&content),
        "ts" | "tsx" | "mts" | "cts" => extract_typescript_imports(&content),
        "js" | "jsx" | "mjs" | "cjs" => extract_javascript_imports(&content),
        "py" => extract_python_imports(&content),
        "go" => extract_go_imports(&content),
        _ => None,
    };

    imports.filter(|i| !i.is_empty())
}

// =============================================================================
// Rust import extraction
// =============================================================================

static RUST_USE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^use\s+([^;]+);").unwrap());

fn extract_rust_imports(content: &str) -> Option<FileImports> {
    let mut imports = FileImports::default();

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(caps) = RUST_USE.captures(trimmed) {
            if let Some(use_path) = caps.get(1) {
                let path_str = use_path.as_str().trim();
                categorize_rust_import(path_str, &mut imports);
            }
        }
    }

    Some(imports)
}

fn categorize_rust_import(path: &str, imports: &mut FileImports) {
    // Extract the root crate/module name
    let root = path.split("::").next().unwrap_or(path);

    // Handle grouped imports like std::{io, fs}
    if path.contains('{') {
        // For now, just use the root
        let root_name = root.trim();
        if root_name == "std" || root_name == "core" || root_name == "alloc" {
            imports.std.push(simplify_path(path));
        } else if root_name == "crate" || root_name == "self" || root_name == "super" {
            imports.internal.push(simplify_path(path));
        } else {
            imports.external.push(root_name.to_string());
        }
    } else {
        // Simple import
        if root == "std" || root == "core" || root == "alloc" {
            imports.std.push(simplify_path(path));
        } else if root == "crate" || root == "self" || root == "super" {
            imports.internal.push(simplify_path(path));
        } else {
            imports.external.push(root.to_string());
        }
    }
}

/// Simplify a Rust path for display (e.g., std::path::Path -> path::Path)
fn simplify_path(path: &str) -> String {
    // For std/core/alloc/crate/self, remove the prefix
    if let Some(stripped) = path.strip_prefix("std::") {
        stripped.to_string()
    } else if let Some(stripped) = path.strip_prefix("core::") {
        stripped.to_string()
    } else if let Some(stripped) = path.strip_prefix("alloc::") {
        stripped.to_string()
    } else if let Some(stripped) = path.strip_prefix("crate::") {
        stripped.to_string()
    } else if let Some(stripped) = path.strip_prefix("self::") {
        stripped.to_string()
    } else {
        // Keep super:: and other prefixes for clarity
        path.to_string()
    }
}

// =============================================================================
// TypeScript/JavaScript import extraction
// =============================================================================

static TS_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"import\s+(?:[^'"]+\s+from\s+)?['"]([^'"]+)['"]"#).unwrap());

static JS_REQUIRE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap());

static TS_EXPORT_FROM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"export\s+(?:\*|\{[^}]*\})\s+from\s+['"]([^'"]+)['"]"#).unwrap());

// Node.js built-in modules
const NODE_BUILTINS: &[&str] = &[
    "assert",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "https",
    "module",
    "net",
    "os",
    "path",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "timers",
    "tls",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "zlib",
];

fn extract_typescript_imports(content: &str) -> Option<FileImports> {
    let mut imports = FileImports::default();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check import statements
        if let Some(caps) = TS_IMPORT.captures(trimmed) {
            if let Some(module) = caps.get(1) {
                categorize_js_import(module.as_str(), &mut imports);
            }
        }

        // Check export from statements
        if let Some(caps) = TS_EXPORT_FROM.captures(trimmed) {
            if let Some(module) = caps.get(1) {
                categorize_js_import(module.as_str(), &mut imports);
            }
        }
    }

    Some(imports)
}

fn extract_javascript_imports(content: &str) -> Option<FileImports> {
    let mut imports = FileImports::default();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check ES6 import statements
        if let Some(caps) = TS_IMPORT.captures(trimmed) {
            if let Some(module) = caps.get(1) {
                categorize_js_import(module.as_str(), &mut imports);
            }
        }

        // Check require() calls
        if let Some(caps) = JS_REQUIRE.captures(trimmed) {
            if let Some(module) = caps.get(1) {
                categorize_js_import(module.as_str(), &mut imports);
            }
        }

        // Check export from statements
        if let Some(caps) = TS_EXPORT_FROM.captures(trimmed) {
            if let Some(module) = caps.get(1) {
                categorize_js_import(module.as_str(), &mut imports);
            }
        }
    }

    Some(imports)
}

fn categorize_js_import(module: &str, imports: &mut FileImports) {
    // Relative imports
    if module.starts_with("./") || module.starts_with("../") {
        imports.internal.push(module.to_string());
    }
    // Node.js builtins (with or without node: prefix)
    else if let Some(stripped) = module.strip_prefix("node:") {
        imports.std.push(stripped.to_string());
    } else if NODE_BUILTINS.contains(&module) {
        imports.std.push(module.to_string());
    }
    // Scoped packages like @types/node
    else if module.starts_with('@') {
        // Get just the package name (e.g., @types/node -> @types/node)
        let pkg = module.split('/').take(2).collect::<Vec<_>>().join("/");
        if !imports.external.contains(&pkg) {
            imports.external.push(pkg);
        }
    }
    // Regular npm packages
    else {
        // Get just the package name (e.g., lodash/fp -> lodash)
        let pkg = module.split('/').next().unwrap_or(module);
        if !imports.external.contains(&pkg.to_string()) {
            imports.external.push(pkg.to_string());
        }
    }
}

// =============================================================================
// Python import extraction
// =============================================================================

static PY_IMPORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^import\s+(\w+)").unwrap());

static PY_FROM_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^from\s+(\.*)(\w+)?").unwrap());

// Python standard library modules (comprehensive list of top-level modules)
const PYTHON_STDLIB: &[&str] = &[
    "abc",
    "aifc",
    "argparse",
    "array",
    "ast",
    "asynchat",
    "asyncio",
    "asyncore",
    "atexit",
    "audioop",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cgi",
    "cgitb",
    "chunk",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "cProfile",
    "crypt",
    "csv",
    "ctypes",
    "curses",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "distutils",
    "doctest",
    "email",
    "encodings",
    "enum",
    "errno",
    "faulthandler",
    "fcntl",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "grp",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "imaplib",
    "imghdr",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "mailcap",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "multiprocessing",
    "netrc",
    "nntplib",
    "numbers",
    "operator",
    "optparse",
    "os",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pipes",
    "pkgutil",
    "platform",
    "plistlib",
    "poplib",
    "posix",
    "pprint",
    "profile",
    "pstats",
    "pty",
    "pwd",
    "pyclbr",
    "pydoc",
    "queue",
    "quopri",
    "random",
    "re",
    "readline",
    "reprlib",
    "resource",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtpd",
    "smtplib",
    "sndhdr",
    "socket",
    "socketserver",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "sunau",
    "symtable",
    "sys",
    "sysconfig",
    "syslog",
    "tabnanny",
    "tarfile",
    "telnetlib",
    "tempfile",
    "termios",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "tomllib",
    "trace",
    "traceback",
    "tracemalloc",
    "tty",
    "turtle",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uu",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "winreg",
    "winsound",
    "wsgiref",
    "xdrlib",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];

fn extract_python_imports(content: &str) -> Option<FileImports> {
    let mut imports = FileImports::default();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Check "import X" statements
        if let Some(caps) = PY_IMPORT.captures(trimmed) {
            if let Some(module) = caps.get(1) {
                categorize_python_import(module.as_str(), false, &mut imports);
            }
        }

        // Check "from X import Y" statements
        if let Some(caps) = PY_FROM_IMPORT.captures(trimmed) {
            let dots = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let module = caps.get(2).map(|m| m.as_str());

            if !dots.is_empty() {
                // Relative import
                let name = if let Some(m) = module {
                    format!("{}{}", dots, m)
                } else {
                    dots.to_string()
                };
                imports.internal.push(name);
            } else if let Some(m) = module {
                categorize_python_import(m, true, &mut imports);
            }
        }
    }

    Some(imports)
}

fn categorize_python_import(module: &str, _is_from: bool, imports: &mut FileImports) {
    if PYTHON_STDLIB.contains(&module) {
        if !imports.std.contains(&module.to_string()) {
            imports.std.push(module.to_string());
        }
    } else if !imports.external.contains(&module.to_string()) {
        imports.external.push(module.to_string());
    }
}

// =============================================================================
// Go import extraction
// =============================================================================

static GO_IMPORT_SINGLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^import\s+"([^"]+)""#).unwrap());

static GO_IMPORT_BLOCK_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*(?:\w+\s+)?"([^"]+)""#).unwrap());

fn extract_go_imports(content: &str) -> Option<FileImports> {
    let mut imports = FileImports::default();
    let mut in_import_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Single-line import
        if let Some(caps) = GO_IMPORT_SINGLE.captures(trimmed) {
            if let Some(pkg) = caps.get(1) {
                categorize_go_import(pkg.as_str(), &mut imports);
            }
        }

        // Import block start
        if trimmed.starts_with("import (") {
            in_import_block = true;
            continue;
        }

        // Import block end
        if in_import_block && trimmed == ")" {
            in_import_block = false;
            continue;
        }

        // Inside import block
        if in_import_block {
            if let Some(caps) = GO_IMPORT_BLOCK_LINE.captures(trimmed) {
                if let Some(pkg) = caps.get(1) {
                    categorize_go_import(pkg.as_str(), &mut imports);
                }
            }
        }
    }

    Some(imports)
}

fn categorize_go_import(pkg: &str, imports: &mut FileImports) {
    // Go standard library doesn't have dots in path
    if !pkg.contains('.') && !pkg.contains('/') {
        if !imports.std.contains(&pkg.to_string()) {
            imports.std.push(pkg.to_string());
        }
    }
    // Internal package (same module)
    else if pkg.contains("/internal/") || pkg.starts_with("internal/") {
        imports.internal.push(pkg.to_string());
    }
    // External package
    else {
        // Get the main package identifier (e.g., github.com/user/repo -> github.com/user/repo)
        // Take up to the third segment for typical Go module paths
        let parts: Vec<&str> = pkg.split('/').collect();
        let key = if parts.len() >= 3 {
            parts[..3].join("/")
        } else {
            pkg.to_string()
        };
        if !imports.external.contains(&key) {
            imports.external.push(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_imports() {
        let content = r#"
use std::path::Path;
use std::io::{self, Read, Write};
use clap::Parser;
use serde::{Serialize, Deserialize};
use crate::git::GitFilter;
use super::config;
"#;
        let imports = extract_rust_imports(content).unwrap();
        assert!(imports.std.iter().any(|s| s.contains("path")));
        assert!(imports.std.iter().any(|s| s.contains("io")));
        assert!(imports.external.contains(&"clap".to_string()));
        assert!(imports.external.contains(&"serde".to_string()));
        assert!(imports.internal.iter().any(|s| s.contains("git")));
    }

    #[test]
    fn test_typescript_imports() {
        let content = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as path from 'path';
import { MyComponent } from './components';
import type { Config } from '@types/node';
import lodash from 'lodash';
"#;
        let imports = extract_typescript_imports(content).unwrap();
        assert!(imports.external.contains(&"react".to_string()));
        assert!(imports.external.contains(&"lodash".to_string()));
        assert!(imports.external.contains(&"@types/node".to_string()));
        assert!(imports.std.contains(&"path".to_string()));
        assert!(imports.internal.contains(&"./components".to_string()));
    }

    #[test]
    fn test_python_imports() {
        let content = r#"
import os
import sys
from pathlib import Path
from collections import defaultdict
import numpy as np
from . import utils
from ..config import settings
"#;
        let imports = extract_python_imports(content).unwrap();
        assert!(imports.std.contains(&"os".to_string()));
        assert!(imports.std.contains(&"sys".to_string()));
        assert!(imports.std.contains(&"pathlib".to_string()));
        assert!(imports.std.contains(&"collections".to_string()));
        assert!(imports.external.contains(&"numpy".to_string()));
        assert!(imports.internal.iter().any(|s| s.starts_with('.')));
    }

    #[test]
    fn test_go_imports() {
        let content = r#"
package main

import "fmt"

import (
    "os"
    "path/filepath"
    "github.com/spf13/cobra"
    "github.com/user/repo/internal/config"
)
"#;
        let imports = extract_go_imports(content).unwrap();
        assert!(imports.std.contains(&"fmt".to_string()));
        assert!(imports.std.contains(&"os".to_string()));
        assert!(
            imports
                .external
                .contains(&"github.com/spf13/cobra".to_string())
        );
    }

    #[test]
    fn test_imports_summary() {
        let imports = FileImports {
            external: vec!["clap".to_string(), "serde".to_string()],
            std: vec!["path".to_string(), "io".to_string()],
            internal: vec!["git".to_string()],
        };
        let summary = imports.summary();
        assert!(summary.contains("clap"));
        assert!(summary.contains("serde"));
        assert!(summary.contains("std::{path, io}"));
        assert!(summary.contains("crate::{git}"));
    }
}
