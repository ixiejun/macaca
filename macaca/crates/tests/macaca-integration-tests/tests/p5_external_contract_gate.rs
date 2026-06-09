//! P5 §9.7 — external contract non-regression gate.
//!
//! Validates that public HTTP/SSE/manifest/session contracts remain wired after
//! microkernel refactor. Uses static source markers (fast) plus a no-network
//! pipeline dry-run (route-c baseline) so CI catches regressions without LLM keys.
//!
//! Design pattern: **Specification by Example** — required route markers and the
//! autonomous pipeline dry-run are the living contract for shell-facing APIs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Bootstrap must continue to register the v2 chat API surface.
const CHAT_V2_ROUTE_MARKER: &str = "/api/chat/v2";

/// SSE event channel wiring must remain present in the web shell composition layer.
const SSE_ROUTE_MARKER: &str = "sse";

/// Session isolation entry points must remain in the session module tree.
const SESSION_MODULE_MARKER: &str = "crates/shells/macaca-web/src/session/";

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

fn read_workspace_file(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Static contract: v2 chat route remains registered in web bootstrap.
#[test]
fn p5_external_contract_chat_v2_route_registered() {
    let bootstrap = read_workspace_file("crates/shells/macaca-web/src/bootstrap.rs");
    assert!(
        bootstrap.contains(CHAT_V2_ROUTE_MARKER),
        "bootstrap must register {CHAT_V2_ROUTE_MARKER} for external chat contract"
    );
    eprintln!("p5_external_contract_gate event=chat_v2_route_pass");
}

/// Static contract: session module tree remains the isolation seam for chat sessions.
#[test]
fn p5_external_contract_session_module_present() {
    let session_mod = workspace_root().join("crates/shells/macaca-web/src/session/mod.rs");
    assert!(
        session_mod.is_file(),
        "session module must exist at {}",
        session_mod.display()
    );
    let lib = read_workspace_file("crates/shells/macaca-web/src/lib.rs");
    assert!(
        lib.contains("mod session") || lib.contains("pub mod session"),
        "macaca-web lib must export session module for session isolation contract"
    );
    eprintln!(
        "p5_external_contract_gate event=session_module_pass path={SESSION_MODULE_MARKER}"
    );
}

/// Static contract: SSE wiring remains in routes/composition (substring guard).
#[test]
fn p5_external_contract_sse_surface_present() {
    let routes = read_workspace_file("crates/shells/macaca-web/src/routes/mod.rs");
    assert!(
        routes.to_lowercase().contains(SSE_ROUTE_MARKER),
        "routes module must retain SSE surface for streaming contract"
    );
    eprintln!("p5_external_contract_gate event=sse_surface_pass");
}

/// Dynamic contract: no-network autonomous pipeline (route-c baseline) still passes.
#[test]
fn p5_external_contract_route_c_no_network_pipeline_passes() {
    eprintln!("p5_external_contract_gate event=route_c_subprocess_start");
    let output = Command::new("cargo")
        .args([
            "test",
            "-p",
            "macaca-integration-tests",
            "route_c_baseline_no_network_pipeline_still_passes",
            "--",
            "--nocapture",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("cargo test subprocess should spawn");

    assert!(
        output.status.success(),
        "route-c no-network pipeline regression:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("p5_external_contract_gate event=route_c_subprocess_pass");
}
