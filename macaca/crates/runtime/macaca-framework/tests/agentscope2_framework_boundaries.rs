// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_CARGO_TERMS: &[&str] =
    &["macaca-compat", "macaca-sdk", "macaca-llm", "macaca-tools"];

const FORBIDDEN_PRODUCTION_TERMS: &[&str] = &[
    "macaca-compat",
    "adapter_llm",
    "macaca_sdk",
    "macaca_llm",
    "macaca_tools",
    "ReActAgent2",
    "AgentRuntime2",
    "AgentScope2RuntimeProvider",
    "LegacyToolHandlerAdapter",
    "from_legacy_response",
    "#[deprecated",
];

#[test]
fn framework_cargo_has_no_concrete_provider_or_compat_features() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("framework Cargo.toml should be readable");

    for term in FORBIDDEN_CARGO_TERMS {
        assert!(
            !cargo_toml.contains(term),
            "macaca-framework Cargo.toml must not contain concrete provider/service or compat term `{term}`"
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
        for term in FORBIDDEN_PRODUCTION_TERMS {
            if source.contains(term) {
                violations.push(format!("{} contains `{term}`", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "framework production source contains forbidden AgentScope 1.0/compat fallback markers:\n{}",
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
