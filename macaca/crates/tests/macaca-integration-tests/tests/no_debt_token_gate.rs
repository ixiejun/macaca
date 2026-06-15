//! Repository terminal debt-token gate.
//!
//! This gate scans Rust sources that participate in OS-layer production and
//! integration contracts. It rejects retired Macaca path markers while allowing
//! third-party wire-protocol names such as OpenAI wire-aligned endpoints.

use std::path::{Path, PathBuf};

/// One forbidden text pattern assembled without embedding retired words.
struct DebtPattern {
    label: &'static str,
    needle: String,
    rationale: &'static str,
    requires_identifier_boundary: bool,
}

/// One deterministic scan violation rendered in sorted order.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DebtViolation {
    label: &'static str,
    path: String,
    line: usize,
    rationale: &'static str,
}

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn joined(parts: &[&str]) -> String {
    parts.concat()
}

fn debt_patterns() -> Vec<DebtPattern> {
    vec![
        DebtPattern {
            label: "retired-path-marker",
            needle: joined(&["leg", "acy"]),
            rationale: "retired path markers must not remain in active Rust sources",
            requires_identifier_boundary: false,
        },
        DebtPattern {
            label: "bridge-marker",
            needle: joined(&["com", "pat"]),
            rationale: "bridge markers must not describe active Macaca OS paths",
            requires_identifier_boundary: false,
        },
        DebtPattern {
            label: "phase-route-marker",
            needle: joined(&["Route", " C"]),
            rationale: "historical phase names must not be active architecture terms",
            requires_identifier_boundary: false,
        },
        DebtPattern {
            label: "phase-route-symbol",
            needle: joined(&["route", "_c"]),
            rationale: "historical phase symbols must not be active Rust identifiers",
            requires_identifier_boundary: true,
        },
        DebtPattern {
            label: "retired-attribute",
            needle: joined(&["#[", "depre", "cated"]),
            rationale: "active Rust sources must not expose retired wrapper APIs",
            requires_identifier_boundary: false,
        },
        DebtPattern {
            label: "retired-allow-attribute",
            needle: joined(&["#[allow(", "depre", "cated", ")"]),
            rationale: "active Rust sources must not suppress retired wrapper API warnings",
            requires_identifier_boundary: false,
        },
    ]
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn is_third_party_wire_protocol_exception(line: &str) -> bool {
    let openai_wire = joined(&["OpenAI-", "com", "pat", "ible"]);
    let openai_symbol = joined(&["openai_", "com", "pat", "ible"]);
    let anthro_wire = joined(&["Anthropic-", "com", "pat", "ible"]);
    let official_dashscope_path = joined(&["com", "pat", "ible-mode"]);
    line.contains(&openai_wire)
        || line.contains(&openai_symbol)
        || line.contains(&anthro_wire)
        || line.contains(&official_dashscope_path)
        || (line.to_ascii_lowercase().contains("dashscope")
            && line.contains(&joined(&["com", "pat"])))
}

fn is_gate_source(relative: &str) -> bool {
    relative.ends_with("no_debt_token_gate.rs")
}

fn is_rust_identifier_char(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn contains_pattern(line: &str, pattern: &DebtPattern) -> bool {
    if !pattern.requires_identifier_boundary {
        return line.contains(&pattern.needle);
    }

    let mut search_start = 0;
    while let Some(offset) = line[search_start..].find(&pattern.needle) {
        let start = search_start + offset;
        let end = start + pattern.needle.len();
        let before_is_identifier = line[..start]
            .chars()
            .next_back()
            .is_some_and(is_rust_identifier_char);
        let after_is_identifier = line[end..]
            .chars()
            .next()
            .is_some_and(is_rust_identifier_char);
        if !before_is_identifier && !after_is_identifier {
            return true;
        }
        search_start = end;
    }
    false
}

fn scan_debt_tokens() -> Vec<DebtViolation> {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_rust_files(&root.join("crates"), &mut files);
    files.sort();

    let patterns = debt_patterns();
    let mut violations = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_gate_source(&relative) {
            continue;
        }

        let content = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in content.lines().enumerate() {
            if is_third_party_wire_protocol_exception(line) {
                continue;
            }
            for pattern in &patterns {
                if contains_pattern(line, pattern) {
                    violations.push(DebtViolation {
                        label: pattern.label,
                        path: relative.clone(),
                        line: index + 1,
                        rationale: pattern.rationale,
                    });
                }
            }
        }
    }
    violations.sort();
    violations
}

fn render_violations(violations: &[DebtViolation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nlabel={}\nfile={}:{}\nrationale={}\n",
                violation.label, violation.path, violation.line, violation.rationale
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn rust_sources_do_not_carry_retired_debt_tokens() {
    let violations = scan_debt_tokens();
    assert!(
        violations.is_empty(),
        "retired Macaca debt tokens remain in Rust sources:{}",
        render_violations(&violations)
    );
}
