//! P5 §9.8 — OpenSpec strict validation terminal gate.
//!
//! CI-discoverable entry point that runs `openspec validate --all --strict` so
//! baseline specs and active change deltas stay aligned with the microkernel
//! refactor terminal state without requiring a global `openspec` PATH install.
//!
//! Design pattern: **Command** — subprocess delegates to `npx @fission-ai/openspec`
//! for hermetic, version-pinned validation identical to local developer workflow.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Relative path to the OpenSpec project root inside the Macaca workspace.
const OPENSPEC_ROOT: &str = "openspec";

fn workspace_root() -> PathBuf {
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

/// Static guard: OpenSpec directory must exist before spawning the CLI subprocess.
fn assert_openspec_root_present(workspace: &Path) {
    eprintln!("openspec_validate_terminal_gate event=openspec_root_check");
    let openspec_dir = workspace.join(OPENSPEC_ROOT);
    assert!(
        openspec_dir.is_dir(),
        "OpenSpec root must exist at {}",
        openspec_dir.display()
    );
    eprintln!("openspec_validate_terminal_gate event=openspec_root_pass");
}

/// Runs pinned OpenSpec CLI validation; panics with combined stdout/stderr on failure.
#[test]
fn openspec_validate_all_strict_terminal_state() {
    let workspace = workspace_root();
    assert_openspec_root_present(&workspace);

    eprintln!("openspec_validate_terminal_gate event=subprocess_start cmd=npx openspec validate --all --strict");
    let output = Command::new("npx")
        .args([
            "--yes",
            "@fission-ai/openspec@latest",
            "validate",
            "--all",
            "--strict",
        ])
        .current_dir(&workspace)
        .output()
        .unwrap_or_else(|error| {
            panic!("failed to spawn npx @fission-ai/openspec validate --all --strict: {error}")
        });

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "openspec validate --all --strict failed (exit {:?})\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );

    eprintln!("openspec_validate_terminal_gate event=subprocess_pass");
    eprintln!("openspec_validate_terminal_gate event=terminal_pass");
}
