// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::{Path, PathBuf};

fn forbidden_cargo_terms() -> Vec<String> {
    vec![
        ["macaca-", "com", "pat"].concat(),
        "macaca-sdk".into(),
        "macaca-llm".into(),
        "macaca-tools".into(),
    ]
}

fn forbidden_production_terms() -> Vec<String> {
    vec![
        ["macaca-", "com", "pat"].concat(),
        "adapter_llm".into(),
        "macaca_sdk".into(),
        "macaca_llm".into(),
        "macaca_tools".into(),
        "ReActAgent2".into(),
        "AgentRuntime2".into(),
        "AgentScope2RuntimeProvider".into(),
        ["Leg", "acyToolHandlerAdapter"].concat(),
        ["from_", "leg", "acy_response"].concat(),
        ["#[", "depre", "cated"].concat(),
    ]
}

#[test]
fn framework_cargo_has_no_concrete_provider_or_forbidden_features() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("framework Cargo.toml should be readable");

    for term in forbidden_cargo_terms() {
        assert!(
            !cargo_toml.contains(&term),
            "macaca-framework Cargo.toml must not contain concrete provider/service forbidden term `{term}`"
        );
    }
}

#[test]
fn framework_source_has_no_agentscope1_or_version_suffixed_runtime_fallbacks() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let mut violations = Vec::new();
    visit_rust_files(&src_dir, &mut |path| {
        if is_test_file(path) {
            return;
        }
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for term in forbidden_production_terms() {
            if source.contains(&term) {
                violations.push(format!("{} contains `{term}`", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "framework production source contains forbidden AgentScope 1.0 fallback markers:\n{}",
        violations.join("\n")
    );
}

fn visit_rust_files(dir: &Path, visitor: &mut impl FnMut(&Path)) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read directory {}: {error}", dir.display()))
    {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            visit_rust_files(&path, visitor);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            visitor(&path);
        }
    }
}

fn is_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("_tests.rs"))
}
