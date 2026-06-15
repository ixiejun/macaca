//! Self-evolving Skill OS boundary gates.
//!
//! These tests are executable architecture specifications for the Skill
//! governance, curation, evolution, semantic review, and mutation surfaces. The
//! dependency gate catches Cargo-level ownership inversions; this file adds a
//! lightweight source scan for semantic leaks that can happen through re-exports,
//! helper names, comments that become copied code, or future alignment
//! modules. The checks stay generic and provider-neutral, so they protect the
//! OS boundary without encoding application behavior.

use std::fs;
use std::path::{Path, PathBuf};

/// A bounded source tree snapshot used by the boundary assertions.
///
/// The scanner ignores build output and hidden folders. It reads only Rust
/// sources because this gate protects Rust crate ownership; frontend source
/// checks should live with the frontend lint/build gates.
#[derive(Debug)]
struct SourceTree {
    root: PathBuf,
    files: Vec<(PathBuf, String)>,
}

impl SourceTree {
    /// Load every Rust source file under `relative_root`.
    fn load(relative_root: &str) -> Self {
        let root = workspace_root().join(relative_root);
        let mut files = Vec::new();
        collect_rust_sources(&root, &mut files);
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Self { root, files }
    }

    /// Assert none of the forbidden tokens appear in the scanned source.
    fn assert_absent(&self, tokens: &[&str]) {
        let mut violations = Vec::new();
        for (path, content) in &self.files {
            for token in tokens {
                if content.contains(token) {
                    violations.push(format!("{} contains `{}`", path.display(), token));
                }
            }
        }
        assert!(
            violations.is_empty(),
            "self-evolving Skill OS boundary violations under {}:\n{}",
            self.root.display(),
            violations.join("\n")
        );
    }
}

#[test]
fn kernel_has_no_self_evolving_skill_semantics() {
    SourceTree::load("crates/kernel/macaca-kernel/src").assert_absent(&[
        "SkillCuration",
        "SkillEvolution",
        "SkillSemanticReview",
        "SkillMutation",
        "SkillGovernanceStore",
        "SkillMergeEligibility",
    ]);
}

#[test]
fn sdk_does_not_construct_skill_runtime_or_store_providers() {
    SourceTree::load("crates/facade/macaca-sdk/src").assert_absent(&[
        "SkillSystemServiceProvider",
        "SkillProviderGovernanceState",
        "SkillGovernanceStoreStrategy",
        "RedbStore::open",
        "PackageGuard",
        "SemanticReviewProviderFactory",
    ]);
}

#[test]
fn presentation_shells_do_not_own_skill_curation_semantics() {
    for shell in [
        "crates/shells/macaca-cli/src",
        "crates/shells/macaca-web/src/skill_operations_routes",
    ] {
        SourceTree::load(shell).assert_absent(&[
            "SkillGovernanceStoreStrategy",
            "SkillProviderGovernanceState",
            "SkillSemanticReviewProvider",
            "SkillMergeEligibilityPolicy",
            "SkillMutationPlan",
            "SkillOwnershipPolicyContext",
        ]);
    }
}

#[test]
fn skill_service_provider_has_no_presentation_shell_dependency() {
    SourceTree::load("crates/services/macaca-skill/src").assert_absent(&[
        "macaca_web::",
        "macaca_cli::",
        "crates/shells",
        "frontend/",
    ]);
}

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn collect_rust_sources(root: &Path, files: &mut Vec<(PathBuf, String)>) {
    if root.is_file() {
        if root.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push((
                root.to_path_buf(),
                fs::read_to_string(root).expect("Rust source should be readable"),
            ));
        }
        return;
    }

    for entry in fs::read_dir(root).expect("source root should be readable") {
        let entry = entry.expect("source entry should be readable");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        collect_rust_sources(&path, files);
    }
}
