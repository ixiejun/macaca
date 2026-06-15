//! Application framework old-helper regression gate.
//!
//! The application framework exposes manifest, ABI, lifecycle, and projection
//! contracts. It must not reintroduce helper APIs that preserve old caller
//! shortcuts after the canonical manifest/projection APIs have replaced them.

use std::path::{Path, PathBuf};

/// One retired application helper and its canonical replacement.
struct RetiredApplicationHelper {
    token: String,
    replacement: &'static str,
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

fn read_workspace_file(relative_path: &str) -> String {
    let path = workspace_root().join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn joined_token(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn application_public_surface_does_not_reintroduce_old_consumption_helpers() {
    let lib_source = read_workspace_file("crates/application/macaca-app/src/lib.rs");
    let consumption_source =
        read_workspace_file("crates/application/macaca-app/src/consumption.rs");
    let combined_source = format!("{lib_source}\n{consumption_source}");
    let retired = [
        RetiredApplicationHelper {
            token: "app_entry_agent_name_or".to_string(),
            replacement: "app_entry_agent_name(manifest).unwrap_or(fallback)",
        },
        RetiredApplicationHelper {
            token: "app_agent_base_prompt".to_string(),
            replacement: "app_agent_prompt_semantics(manifest, agent_name).base_prompt",
        },
        RetiredApplicationHelper {
            token: joined_token(&["leg", "acy_app_task_planning_contract"]),
            replacement: "app_task_planning_contract or explicit ApplicationTaskPlanningContract",
        },
    ];

    let violations = retired
        .iter()
        .filter(|helper| combined_source.contains(&helper.token))
        .map(|helper| format!("{} -> {}", helper.token, helper.replacement))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "application framework must not expose retired consumption helpers:\n{}",
        violations.join("\n")
    );
}
