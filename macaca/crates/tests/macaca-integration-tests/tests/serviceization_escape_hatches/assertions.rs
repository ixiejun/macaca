//! Assertion helpers for retired serviceization escape-hatch policy families.
//!
//! These helpers encode reusable Specification-pattern checks. Individual tests
//! supply rule families, tokens, or explicit paths while this module owns stable
//! scanning, sorting, and failure rendering.

use super::scanner::{
    collect_rust_files, is_non_production_rust_source, render_violations, scan_file,
};
use super::support::workspace_root;
use super::tokens::forbidden_tokens;

pub fn assert_retired_escape_hatch_tokens_absent_in_production(tokens: &[&str]) {
    assert_retired_escape_hatch_tokens_absent_in_production_with_allowed_paths(tokens, &[]);
}

pub fn assert_production_literal_tokens_absent_outside_allowed_paths(
    literals: &[&str],
    allowed_path_prefixes: &[&str],
) {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut hits = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_non_production_rust_source(&relative) {
            continue;
        }
        if allowed_path_prefixes
            .iter()
            .any(|prefix| relative.starts_with(prefix))
        {
            continue;
        }
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in content.lines().enumerate() {
            for literal in literals {
                if line.contains(literal) {
                    hits.push(format!(
                        "\nfamily=literal-guard\nfile={}:{}\ntoken={}\n",
                        relative,
                        index + 1,
                        literal
                    ));
                }
            }
        }
    }
    hits.sort();

    assert!(
        hits.is_empty(),
        "Production literal tokens {:?} must be absent outside {:?}:{}",
        literals,
        allowed_path_prefixes,
        hits.join("")
    );
}

pub fn assert_production_paths_literal_tokens_absent(relative_paths: &[&str], literals: &[&str]) {
    let root = workspace_root();
    let mut hits = Vec::new();

    for relative in relative_paths {
        let path = root.join(relative);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for (index, line) in content.lines().enumerate() {
            for literal in literals {
                if line.contains(literal) {
                    hits.push(format!(
                        "\nfamily=literal-guard\nfile={}:{}\ntoken={}\n",
                        relative,
                        index + 1,
                        literal
                    ));
                }
            }
        }
    }
    hits.sort();

    assert!(
        hits.is_empty(),
        "Production paths {:?} must not contain literal tokens {:?}:{}",
        relative_paths,
        literals,
        hits.join("")
    );
}

pub fn assert_retired_escape_hatch_tokens_absent_in_production_with_allowed_paths(
    tokens: &[&str],
    allowed_path_prefixes: &[&str],
) {
    let retired_tokens = forbidden_tokens()
        .into_iter()
        .filter(|entry| tokens.contains(&entry.token))
        .collect::<Vec<_>>();
    assert!(
        !retired_tokens.is_empty(),
        "retired token list must match at least one forbidden token: {tokens:?}"
    );

    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut violations = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_non_production_rust_source(&relative) {
            continue;
        }
        if allowed_path_prefixes
            .iter()
            .any(|prefix| relative.starts_with(prefix))
        {
            continue;
        }
        violations.extend(scan_file(&root, file, &retired_tokens, false));
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "Retired escape-hatch tokens {:?} must be absent from production code outside {:?} \
         (no terminal exception):{}",
        tokens,
        allowed_path_prefixes,
        render_violations(&violations)
    );
}

pub fn assert_retired_escape_hatch_family_absent_in_production(family: &str) {
    let retired_tokens = forbidden_tokens()
        .into_iter()
        .filter(|token| token.family == family)
        .collect::<Vec<_>>();

    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut violations = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_non_production_rust_source(&relative) {
            continue;
        }
        violations.extend(scan_file(&root, file, &retired_tokens, false));
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "Retired escape-hatch family `{family}` must be absent from all production code \
         (no terminal exception):{}",
        render_violations(&violations)
    );
}
