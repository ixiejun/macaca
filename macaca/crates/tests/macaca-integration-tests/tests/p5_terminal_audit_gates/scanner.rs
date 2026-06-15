//! Shared static scanner for P5 terminal audit gates (§6.1.2–6.1.4).
//!
//! Each gate filters the global forbidden-token catalog by **family** (Strategy)
//! and scans production Rust sources under `crates/`. Terminal exception surfaces from
//! `surfaces.rs` mirror `serviceization_escape_hatches` so P5 gates stay aligned
//! with the protocol microkernel boundary while exposing named VC entry points.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::surfaces::is_approved_terminal_exception_surface;
use super::tokens::{forbidden_tokens, ForbiddenToken};

/// One deterministic violation for stable CI output.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Violation {
    pub gate_id: &'static str,
    pub family: &'static str,
    pub path: PathBuf,
    pub line: usize,
    pub token: &'static str,
    pub rationale: &'static str,
}

pub fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from integration-tests manifest");
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | "tests" | "fixtures" | "examples" | ".git" | ".playwright-mcp"
    )
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if should_skip_dir(name) {
            continue;
        }
        if path.is_dir() {
            collect_rust_files(&path, files);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*")
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn scan_file(
    gate_id: &'static str,
    root: &Path,
    path: &Path,
    tokens: &[ForbiddenToken],
) -> Vec<Violation> {
    let relative = path
        .strip_prefix(root)
        .expect("scanned file should be under workspace root")
        .to_string_lossy()
        .replace('\\', "/");
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut violations = Vec::new();
    let mut pending_cfg_test = false;
    let mut test_module_depth: Option<i32> = None;

    for (index, line) in content.lines().enumerate() {
        if let Some(depth) = test_module_depth {
            let next_depth = depth + brace_delta(line);
            if next_depth <= 0 {
                test_module_depth = None;
            } else {
                test_module_depth = Some(next_depth);
            }
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            continue;
        }
        if pending_cfg_test {
            if trimmed.starts_with("mod tests") || trimmed.starts_with("pub mod tests") {
                let depth = brace_delta(line);
                if depth > 0 {
                    test_module_depth = Some(depth);
                }
                pending_cfg_test = false;
                continue;
            }
            if !trimmed.starts_with("#[") && !trimmed.is_empty() {
                pending_cfg_test = false;
            }
        }

        for token in tokens {
            if !line.contains(token.token)
                || (token.family == "hardcoded-agent-role" && is_comment_only_line(line))
                || is_approved_terminal_exception_surface(&relative, token)
            {
                continue;
            }
            violations.push(Violation {
                gate_id,
                family: token.family,
                path: PathBuf::from(&relative),
                line: index + 1,
                token: token.token,
                rationale: token.rationale,
            });
        }
    }

    violations
}

fn render_violations(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\ngate={}\nfamily={}\nfile={}:{}\ntoken={}\nrationale={}\nprocess=Route through the service client/facade for this capability, or update the terminal specification with a justified exception.\n",
                violation.gate_id,
                violation.family,
                violation.path.display(),
                violation.line,
                violation.token,
                violation.rationale
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Runs one named P5 audit gate over the selected token families.
///
/// `path_prefix_filter`, when `Some`, restricts hits to files whose relative path
/// starts with the prefix — used by shell-specific gates without duplicating tokens.
pub fn assert_token_family_gate(
    gate_id: &'static str,
    families: &[&str],
    path_prefix_filter: Option<&str>,
) {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let all_tokens = forbidden_tokens();
    let tokens: Vec<ForbiddenToken> = all_tokens
        .into_iter()
        .filter(|token| families.contains(&token.family))
        .collect();

    eprintln!(
        "p5_terminal_audit_gate event=scan_start gate={gate_id} families={families:?} tokens={}",
        tokens.len()
    );

    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        violations.extend(scan_file(gate_id, &root, file, &tokens));
    }
    if let Some(prefix) = path_prefix_filter {
        violations.retain(|violation| {
            violation
                .path
                .to_string_lossy()
                .replace('\\', "/")
                .starts_with(prefix)
        });
    }
    violations.sort();

    eprintln!(
        "p5_terminal_audit_gate event=scan_complete gate={gate_id} files={} violations={}",
        files.len(),
        violations.len()
    );

    assert!(
        violations.is_empty(),
        "P5 terminal audit gate `{gate_id}` violations were found:{}",
        render_violations(&violations)
    );
}
