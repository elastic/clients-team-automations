use std::path::{Component, PathBuf};

#[derive(Debug, Clone)]
pub struct ReferencedPathData {
    pub raw_path: String,
    pub resolved_path: Option<String>,
    pub kind: String,
    pub line_number: i64,
    pub source_path: String,
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Resolve a relative `raw_path` against the directory containing `source_file_rel_path`
/// (which is repo-root-relative). Returns a normalized repo-root-relative path, or `None`
/// if the resolved path would escape the repo root.
pub fn resolve_path(raw_path: &str, source_file_rel_path: &str) -> Option<String> {
    let source_dir = std::path::Path::new(source_file_rel_path).parent()?;
    let joined = source_dir.join(raw_path);

    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
            Component::Normal(seg) => normalized.push(seg),
            _ => {}
        }
    }

    let s = normalized.to_string_lossy().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

// ---------------------------------------------------------------------------
// Markdown link conversion
// ---------------------------------------------------------------------------

/// Convert raw markdown link data (produced by the markdown parser) into
/// `ReferencedPathData`, filtering out non-local destinations.
pub fn referenced_paths_from_markdown_links(
    links: &[crate::markdown::MarkdownLinkData],
    skill_file_rel_path: &str,
) -> Vec<ReferencedPathData> {
    links
        .iter()
        .filter(|link| is_local_path(&link.dest_url))
        .map(|link| {
            let kind = if link.is_image {
                "markdown_image"
            } else {
                "markdown_link"
            };
            let resolved = resolve_path(&link.dest_url, skill_file_rel_path);
            ReferencedPathData {
                raw_path: link.dest_url.clone(),
                resolved_path: resolved,
                kind: kind.to_string(),
                line_number: link.line_number,
                source_path: skill_file_rel_path.to_string(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Script / file reference extraction
// ---------------------------------------------------------------------------

/// Extract local path references from a file's content, dispatching by extension.
pub fn extract_file_references(
    content: &str,
    extension: &str,
    file_rel_path: &str,
) -> Vec<ReferencedPathData> {
    match extension {
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" => {
            extract_js_references(content, file_rel_path)
        }
        "py" => extract_python_references(content, file_rel_path),
        "sh" | "bash" | "zsh" => extract_shell_references(content, file_rel_path),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// JS / TS
// ---------------------------------------------------------------------------

fn extract_js_references(content: &str, file_rel_path: &str) -> Vec<ReferencedPathData> {
    let mut results = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_number = (line_idx + 1) as i64;
        let trimmed = line.trim();

        // `import ... from '...'` or `import ... from "..."`
        if let Some(path) = extract_js_import_from(trimmed) {
            if is_local_path(path) {
                results.push(ReferencedPathData {
                    raw_path: path.to_string(),
                    resolved_path: resolve_path(path, file_rel_path),
                    kind: "js_import".to_string(),
                    line_number,
                    source_path: file_rel_path.to_string(),
                });
            }
            continue;
        }

        // `require('...')` or `require("...")`
        for (path, kind) in extract_js_require_or_dynamic(trimmed) {
            if is_local_path(path) {
                results.push(ReferencedPathData {
                    raw_path: path.to_string(),
                    resolved_path: resolve_path(path, file_rel_path),
                    kind: kind.to_string(),
                    line_number,
                    source_path: file_rel_path.to_string(),
                });
            }
        }
    }

    results
}

/// Match `import ... from 'path'` or `import ... from "path"`
/// Also handles `export ... from 'path'`
fn extract_js_import_from(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("import ")
        .or_else(|| line.strip_prefix("export "))
        .or_else(|| {
            // Handle `import{` (no space)
            if line.starts_with("import{") || line.starts_with("import*") {
                Some(&line[6..])
            } else {
                None
            }
        })?;

    let from_idx = rest.find(" from ")?;
    let after_from = rest[from_idx + 6..].trim();
    extract_quoted_string(after_from)
}

/// Find `require('...')` and `import('...')` calls in a line.
/// Returns (path, kind) pairs.
fn extract_js_require_or_dynamic(line: &str) -> Vec<(&str, &str)> {
    let mut results = Vec::new();
    let patterns: &[(&str, &str)] = &[
        ("require(", "js_require"),
        ("import(", "js_dynamic_import"),
    ];

    for &(pattern, kind) in patterns {
        let mut search = line;
        while let Some(pos) = search.find(pattern) {
            let after = &search[pos + pattern.len()..];
            if let Some(path) = extract_quoted_string(after.trim()) {
                results.push((path, kind));
            }
            search = &search[pos + pattern.len()..];
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

fn extract_python_references(content: &str, file_rel_path: &str) -> Vec<ReferencedPathData> {
    let mut results = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_number = (line_idx + 1) as i64;
        let trimmed = line.trim();

        // `from .foo.bar import baz` -- relative imports
        if let Some(module_path) = extract_python_relative_import(trimmed) {
            let file_path = python_relative_import_to_path(module_path);
            if let Some(ref path_str) = file_path {
                results.push(ReferencedPathData {
                    raw_path: module_path.to_string(),
                    resolved_path: resolve_path(path_str, file_rel_path),
                    kind: "python_relative_import".to_string(),
                    line_number,
                    source_path: file_rel_path.to_string(),
                });
            }
        }

        // `open('...')` or `Path('...')` with relative paths
        for path in extract_python_path_calls(trimmed) {
            if is_local_path(path) {
                results.push(ReferencedPathData {
                    raw_path: path.to_string(),
                    resolved_path: resolve_path(path, file_rel_path),
                    kind: "python_relative_import".to_string(),
                    line_number,
                    source_path: file_rel_path.to_string(),
                });
            }
        }
    }

    results
}

/// Match `from .something import ...` or `from ..something import ...`
fn extract_python_relative_import(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("from ")?;
    if !rest.starts_with('.') {
        return None;
    }
    let end = rest.find(" import ")?;
    let module = rest[..end].trim();
    if module.is_empty() {
        return None;
    }
    Some(module)
}

/// Convert a Python relative import path like `..utils.helpers` to a filesystem
/// relative path like `../utils/helpers`.
fn python_relative_import_to_path(module_path: &str) -> Option<String> {
    let dot_count = module_path.chars().take_while(|&c| c == '.').count();
    if dot_count == 0 {
        return None;
    }

    let module_part = &module_path[dot_count..];
    let mut path = String::new();

    // Each leading dot beyond the first represents a `../`
    // The first dot means "current package" = `./`
    if dot_count == 1 {
        path.push_str("./");
    } else {
        for _ in 0..(dot_count - 1) {
            path.push_str("../");
        }
    }

    if !module_part.is_empty() {
        path.push_str(&module_part.replace('.', "/"));
    }

    Some(path)
}

/// Find `open('...')` and `Path('...')` calls with relative paths.
fn extract_python_path_calls(line: &str) -> Vec<&str> {
    let mut results = Vec::new();
    let patterns = ["open(", "Path("];

    for pattern in &patterns {
        let mut search = line;
        while let Some(pos) = search.find(pattern) {
            let after = &search[pos + pattern.len()..];
            if let Some(path) = extract_quoted_string(after.trim()) {
                results.push(path);
            }
            search = &search[pos + pattern.len()..];
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

fn extract_shell_references(content: &str, file_rel_path: &str) -> Vec<ReferencedPathData> {
    let mut results = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_number = (line_idx + 1) as i64;
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(path) = extract_shell_source(trimmed) {
            if is_local_path(path) {
                results.push(ReferencedPathData {
                    raw_path: path.to_string(),
                    resolved_path: resolve_path(path, file_rel_path),
                    kind: "shell_source".to_string(),
                    line_number,
                    source_path: file_rel_path.to_string(),
                });
            }
        }
    }

    results
}

/// Match `source <path>` or `. <path>` (dot-space-path).
fn extract_shell_source(line: &str) -> Option<&str> {
    let rest = if let Some(r) = line.strip_prefix("source ") {
        r
    } else if let Some(r) = line.strip_prefix(". ") {
        // Make sure it's `. path` not `.. path`
        if line.starts_with(".. ") {
            return None;
        }
        r
    } else {
        return None;
    };

    let path = rest.trim();
    // Strip quotes if present
    let path = path
        .strip_prefix('"')
        .and_then(|p| p.strip_suffix('"'))
        .or_else(|| path.strip_prefix('\'').and_then(|p| p.strip_suffix('\'')))
        .unwrap_or(path);

    let path = path.split_whitespace().next()?;
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if a path looks like a local filesystem reference (not a URL or bare package name).
fn is_local_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("mailto:")
        || path.starts_with("data:")
        || path.starts_with('#')
        || path.starts_with('@')
    {
        return false;
    }
    path.starts_with('.')
        || path.starts_with('/')
        || path.contains('/')
}

/// Extract the content of a single- or double-quoted string at the start of `s`.
fn extract_quoted_string(s: &str) -> Option<&str> {
    let quote = s.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &s[1..];
    let end = rest.find(quote)?;
    let inner = &rest[..end];
    if inner.is_empty() {
        return None;
    }
    Some(inner)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- resolve_path --

    #[test]
    fn resolve_simple_relative() {
        let result = resolve_path("../shared/foo.js", "skills/es/my-skill/scripts/main.js");
        assert_eq!(result, Some("skills/es/my-skill/shared/foo.js".to_string()));
    }

    #[test]
    fn resolve_double_parent() {
        let result = resolve_path(
            "../../shared/shared-skill/data.json",
            "skills/es/my-skill/scripts/main.js",
        );
        assert_eq!(
            result,
            Some("skills/es/shared/shared-skill/data.json".to_string())
        );
    }

    #[test]
    fn resolve_escapes_repo_root() {
        let result = resolve_path("../../../../escape.txt", "skills/es/my-skill/SKILL.md");
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_dot_slash() {
        let result = resolve_path("./helper.py", "skills/es/my-skill/scripts/main.py");
        assert_eq!(
            result,
            Some("skills/es/my-skill/scripts/helper.py".to_string())
        );
    }

    // -- is_local_path --

    #[test]
    fn local_path_relative() {
        assert!(is_local_path("./foo"));
        assert!(is_local_path("../bar"));
        assert!(is_local_path("scripts/run.sh"));
    }

    #[test]
    fn non_local_paths() {
        assert!(!is_local_path("https://example.com"));
        assert!(!is_local_path("http://foo"));
        assert!(!is_local_path("mailto:a@b.com"));
        assert!(!is_local_path("#anchor"));
        assert!(!is_local_path(""));
        assert!(!is_local_path("lodash"));
    }

    // -- JS extraction --

    #[test]
    fn js_import_from() {
        let content = r#"import { createClient } from '../../shared/es-client.js';"#;
        let refs = extract_js_references(content, "skills/es/my-skill/scripts/index.js");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "../../shared/es-client.js");
        assert_eq!(refs[0].kind, "js_import");
        assert_eq!(refs[0].line_number, 1);
        assert_eq!(
            refs[0].resolved_path.as_deref(),
            Some("skills/es/shared/es-client.js")
        );
    }

    #[test]
    fn js_require() {
        let content = "const x = require('./local-mod');";
        let refs = extract_js_references(content, "skills/es/my-skill/scripts/index.js");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "./local-mod");
        assert_eq!(refs[0].kind, "js_require");
    }

    #[test]
    fn js_dynamic_import() {
        let content = r#"const mod = await import("../utils/helper.js");"#;
        let refs = extract_js_references(content, "skills/es/my-skill/scripts/index.js");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "../utils/helper.js");
        assert_eq!(refs[0].kind, "js_dynamic_import");
    }

    #[test]
    fn js_skips_packages() {
        let content = "import lodash from 'lodash';\nconst x = require('@elastic/elasticsearch');";
        let refs = extract_js_references(content, "skills/es/my-skill/scripts/index.js");
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn js_export_from() {
        let content = "export { helper } from '../shared/helpers.js';";
        let refs = extract_js_references(content, "skills/es/my-skill/scripts/index.js");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, "js_import");
        assert_eq!(refs[0].raw_path, "../shared/helpers.js");
    }

    // -- Python extraction --

    #[test]
    fn python_relative_import() {
        let content = "from ..utils.helpers import do_thing";
        let refs = extract_python_references(content, "skills/es/my-skill/scripts/main.py");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "..utils.helpers");
        assert_eq!(refs[0].kind, "python_relative_import");
        assert_eq!(
            refs[0].resolved_path.as_deref(),
            Some("skills/es/my-skill/utils/helpers")
        );
    }

    #[test]
    fn python_single_dot_import() {
        let content = "from .sibling import func";
        let refs = extract_python_references(content, "skills/es/my-skill/scripts/main.py");
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved_path.as_deref(),
            Some("skills/es/my-skill/scripts/sibling")
        );
    }

    #[test]
    fn python_open_call() {
        let content = "data = open('../data/input.json').read()";
        let refs = extract_python_references(content, "skills/es/my-skill/scripts/main.py");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "../data/input.json");
    }

    #[test]
    fn python_skips_absolute_import() {
        let content = "from elasticsearch import Elasticsearch";
        let refs = extract_python_references(content, "skills/es/my-skill/scripts/main.py");
        assert_eq!(refs.len(), 0);
    }

    // -- Shell extraction --

    #[test]
    fn shell_source_command() {
        let content = "source ../shared/common.sh";
        let refs = extract_shell_references(content, "skills/es/my-skill/scripts/run.sh");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "../shared/common.sh");
        assert_eq!(refs[0].kind, "shell_source");
    }

    #[test]
    fn shell_dot_command() {
        let content = ". ./helpers.sh";
        let refs = extract_shell_references(content, "skills/es/my-skill/scripts/run.sh");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "./helpers.sh");
    }

    #[test]
    fn shell_skips_comments() {
        let content = "# source ../not-real.sh";
        let refs = extract_shell_references(content, "skills/es/my-skill/scripts/run.sh");
        assert_eq!(refs.len(), 0);
    }

    // -- extract_quoted_string --

    #[test]
    fn quoted_single() {
        assert_eq!(extract_quoted_string("'hello'"), Some("hello"));
    }

    #[test]
    fn quoted_double() {
        assert_eq!(extract_quoted_string("\"world\""), Some("world"));
    }

    #[test]
    fn quoted_empty() {
        assert_eq!(extract_quoted_string("''"), None);
    }

    #[test]
    fn not_quoted() {
        assert_eq!(extract_quoted_string("nope"), None);
    }

    // -- python_relative_import_to_path --

    #[test]
    fn python_single_dot_path() {
        assert_eq!(
            python_relative_import_to_path(".sibling"),
            Some("./sibling".to_string())
        );
    }

    #[test]
    fn python_double_dot_path() {
        assert_eq!(
            python_relative_import_to_path("..utils.helpers"),
            Some("../utils/helpers".to_string())
        );
    }

    #[test]
    fn python_triple_dot_path() {
        assert_eq!(
            python_relative_import_to_path("...deep.module"),
            Some("../../deep/module".to_string())
        );
    }

    // -- markdown link conversion --

    #[test]
    fn markdown_links_filters_urls() {
        let links = vec![
            crate::markdown::MarkdownLinkData {
                dest_url: "https://example.com".to_string(),
                is_image: false,
                line_number: 1,
            },
            crate::markdown::MarkdownLinkData {
                dest_url: "../shared/ref.md".to_string(),
                is_image: false,
                line_number: 2,
            },
            crate::markdown::MarkdownLinkData {
                dest_url: "#anchor".to_string(),
                is_image: false,
                line_number: 3,
            },
        ];
        let refs = referenced_paths_from_markdown_links(&links, "skills/es/my-skill/SKILL.md");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].raw_path, "../shared/ref.md");
        assert_eq!(refs[0].kind, "markdown_link");
    }

    #[test]
    fn markdown_image_kind() {
        let links = vec![crate::markdown::MarkdownLinkData {
            dest_url: "./assets/diagram.png".to_string(),
            is_image: true,
            line_number: 5,
        }];
        let refs = referenced_paths_from_markdown_links(&links, "skills/es/my-skill/SKILL.md");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, "markdown_image");
    }
}
