//! P5 §9.5 — Definition-of-Done terminal gate matrix implementation.
//!
//! Macaca's microkernel refactor closes when every architectural gate is green
//! and migration allowlists are empty. Individual gate crates encode one concern;
//! this matrix is the **composite specification** that proves the full P5 DoD
//! without requiring operators to memorize fifteen VC commands.
//!
//! Design pattern: **Facade + Command + Registry**
//! - `TerminalGateCommand` entries are immutable registry rows (gate id, task ref,
//!   integration-test target, cargo filter).
//! - Each command spawns `cargo test -p macaca-integration-tests` in a subprocess
//!   so gate modules keep their own `#[path]` trees and `super::` allowlist wiring.
//! - `eprintln!` logs gate_id at start/pass for traceable CI artifacts.
//!
//! Trade-off: subprocess orchestration is slower than in-process calls but avoids
//! fragile cross-gate module linking and matches the pattern used by
//! `unified_audit_replay_terminal_gate` / `audit_blind_spot_terminal_gate`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One registered terminal gate executed via integration-test subprocess.
struct TerminalGateCommand {
    /// Stable gate identifier for logs and failure diagnostics.
    gate_id: &'static str,
    /// OpenSpec tasks.md section reference (audit trail only).
    tasks_ref: &'static str,
    /// `cargo test --test <name>` integration test file stem.
    test_target: &'static str,
    /// Substring filter passed to `cargo test` (matches one or more tests).
    filter: &'static str,
}

/// Canonical registry of P5 terminal gates (order: fast static gates first, heavy
/// behavioral / npx gates last so failures surface quickly in CI).
const TERMINAL_GATE_COMMANDS: &[TerminalGateCommand] = &[
    // §6.1.1 / §9.5 — Route C allowlist terminal state (must be zero rows).
    TerminalGateCommand {
        gate_id: "route_c_allowlist_terminal",
        tasks_ref: "§6.1.1 / §9.5",
        test_target: "route_c_dependency_boundaries",
        filter: "route_c_dependency_allowlist_terminal_state_is_zero_rows",
    },
    // Route C forbidden-edge enforcement (allowlist == 0 implies all edges classified).
    TerminalGateCommand {
        gate_id: "route_c_dependency_boundaries",
        tasks_ref: "§6.1.1",
        test_target: "route_c_dependency_boundaries",
        filter: "route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges",
    },
    // §6.1.6 / §9.6 — OS-layer file size allowlist terminal + boundary scan.
    TerminalGateCommand {
        gate_id: "os_layer_file_size_allowlist_terminal",
        tasks_ref: "§6.1.6 / §9.6",
        test_target: "os_layer_file_size_gate",
        filter: "os_layer_file_size_allowlist_terminal_state_is_zero_rows",
    },
    TerminalGateCommand {
        gate_id: "os_layer_file_size_boundaries",
        tasks_ref: "§6.1.6",
        test_target: "os_layer_file_size_gate",
        filter: "os_layer_file_size_gate_reject_unallowlisted_oversized_sources",
    },
    // §6.1.5 / §9.3 — kernel workspace dependency purity (proto + ipc only).
    TerminalGateCommand {
        gate_id: "kernel_purity",
        tasks_ref: "§6.1.5 / §9.3",
        test_target: "kernel_purity_gate",
        filter: "kernel_purity_gate_rejects_forbidden_workspace_dependencies",
    },
    // §6.1.7 / §9.4 — shell workspace dependency purity (CLI terminal, web frozen baseline).
    TerminalGateCommand {
        gate_id: "shell_dependency_purity_cli",
        tasks_ref: "§6.1.7 / §9.4",
        test_target: "shell_dependency_purity_gate",
        filter: "shell_dependency_purity_gate_cli_is_terminal_proto_sdk_only",
    },
    TerminalGateCommand {
        gate_id: "shell_dependency_purity_web_baseline",
        tasks_ref: "§6.1.7 / §9.4",
        test_target: "shell_dependency_purity_gate",
        filter: "shell_dependency_purity_gate_web_matches_frozen_baseline",
    },
    // §6.1.2–6.1.4 — P5 static audit gates (provider bypass, hardcoded names, shell ownership).
    TerminalGateCommand {
        gate_id: "p5_no_direct_provider_call",
        tasks_ref: "§6.1.2",
        test_target: "p5_terminal_audit_gates",
        filter: "p5_no_direct_provider_call_gate_rejects_bypasses",
    },
    TerminalGateCommand {
        gate_id: "p5_no_hardcoded_name",
        tasks_ref: "§6.1.3",
        test_target: "p5_terminal_audit_gates",
        filter: "p5_no_hardcoded_name_gate_rejects_os_layer_literals",
    },
    TerminalGateCommand {
        gate_id: "p5_shell_not_semantic_owner",
        tasks_ref: "§6.1.4",
        test_target: "p5_terminal_audit_gates",
        filter: "p5_shell_not_semantic_owner_gate_rejects_shell_execution_ownership",
    },
    // §9.2 — multi-path coordination patch retirement.
    TerminalGateCommand {
        gate_id: "p5_coordination_patch_retirement",
        tasks_ref: "§9.2",
        test_target: "p5_terminal_audit_gates",
        filter: "p5_coordination_patch_retirement_gate_rejects_multi_path_markers",
    },
    // §6.2 — escape hatch debt inventory terminal (raw == 0).
    TerminalGateCommand {
        gate_id: "serviceization_escape_hatches_debt_terminal",
        tasks_ref: "§6.2.2",
        test_target: "serviceization_escape_hatches",
        filter: "migration_debt_inventory_matches_baseline",
    },
    TerminalGateCommand {
        gate_id: "serviceization_escape_hatches_reconciliation_absent",
        tasks_ref: "§6.2.2",
        test_target: "serviceization_escape_hatches",
        filter: "serviceization_escape_hatches_reconciliation_markers_absent_in_production",
    },
    // §5.3.2 / §9.6 — domain pack exiled from base runtime-host.
    TerminalGateCommand {
        gate_id: "runtime_host_domain_pack",
        tasks_ref: "§5.3.2 / §9.6",
        test_target: "runtime_host_domain_pack_gate",
        filter: "runtime_host_production_sources_contain_no_finance_domain_tokens",
    },
    // §7.3 — canonical audit redaction module + wiring.
    TerminalGateCommand {
        gate_id: "audit_redaction_terminal",
        tasks_ref: "§7.3",
        test_target: "audit_redaction_terminal_gate",
        filter: "audit_redaction_terminal_state_is_canonical",
    },
    // §9.7 — external HTTP/SSE/session contract (static markers + route-c pipeline).
    TerminalGateCommand {
        gate_id: "p5_external_contract",
        tasks_ref: "§9.7",
        test_target: "p5_external_contract_gate",
        filter: "p5_external_contract",
    },
    // §9.1 — unified audit replay single-chain convergence (subprocess-heavy).
    TerminalGateCommand {
        gate_id: "unified_audit_replay_terminal",
        tasks_ref: "§9.1",
        test_target: "unified_audit_replay_terminal_gate",
        filter: "unified_audit_replay_single_chain_terminal_state",
    },
    // §7.4 — audit blind-spot elimination (replay wiring + runtime-host behavior).
    TerminalGateCommand {
        gate_id: "audit_blind_spot_terminal",
        tasks_ref: "§7.4",
        test_target: "audit_blind_spot_terminal_gate",
        filter: "audit_blind_spot_terminal_state_has_no_replay_gaps",
    },
    // §9.8 — OpenSpec strict validate (npx subprocess inside gate).
    TerminalGateCommand {
        gate_id: "openspec_validate_terminal",
        tasks_ref: "§9.8",
        test_target: "openspec_validate_terminal_gate",
        filter: "openspec_validate_all_strict_terminal_state",
    },
];

/// Integration-test source files that must exist before spawning subprocesses.
const REQUIRED_GATE_TEST_SOURCES: &[&str] = &[
    "crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries.rs",
    "crates/tests/macaca-integration-tests/tests/os_layer_file_size_gate.rs",
    "crates/tests/macaca-integration-tests/tests/kernel_purity_gate.rs",
    "crates/tests/macaca-integration-tests/tests/shell_dependency_purity_gate.rs",
    "crates/tests/macaca-integration-tests/tests/p5_terminal_audit_gates.rs",
    "crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs",
    "crates/tests/macaca-integration-tests/tests/runtime_host_domain_pack_gate.rs",
    "crates/tests/macaca-integration-tests/tests/audit_redaction_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/p5_external_contract_gate.rs",
    "crates/tests/macaca-integration-tests/tests/unified_audit_replay_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/audit_blind_spot_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/openspec_validate_terminal_gate.rs",
];

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

/// Static guard: every registered gate test file must exist on disk.
fn assert_required_gate_sources_present(workspace: &Path) {
    eprintln!(
        "p5_dod_terminal_gate_matrix event=registry_source_check count={}",
        REQUIRED_GATE_TEST_SOURCES.len()
    );
    for relative in REQUIRED_GATE_TEST_SOURCES {
        let path = workspace.join(relative);
        assert!(
            path.is_file(),
            "P5 DoD gate matrix: required integration test missing: {}",
            path.display()
        );
    }
    eprintln!("p5_dod_terminal_gate_matrix event=registry_source_pass");
}

/// Runs one integration-test subprocess for a registered terminal gate command.
fn run_terminal_gate_command(workspace: &Path, command: &TerminalGateCommand) {
    eprintln!(
        "p5_dod_terminal_gate_matrix event=gate_start gate_id={} tasks_ref={} test_target={} filter={}",
        command.gate_id, command.tasks_ref, command.test_target, command.filter
    );

    let output = Command::new("cargo")
        .args([
            "test",
            "-p",
            "macaca-integration-tests",
            "--test",
            command.test_target,
            command.filter,
            "--",
            "--nocapture",
        ])
        .current_dir(workspace)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "P5 DoD gate matrix: failed to spawn cargo test --test {} {}: {error}",
                command.test_target, command.filter
            )
        });

    if output.status.success() {
        eprintln!(
            "p5_dod_terminal_gate_matrix event=gate_pass gate_id={}",
            command.gate_id
        );
        return;
    }

    panic!(
        "P5 DoD gate matrix: gate `{}` failed ({})\n\
         command: cargo test -p macaca-integration-tests --test {} {}\n\
         exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        command.gate_id,
        command.tasks_ref,
        command.test_target,
        command.filter,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Main matrix entry — panics when any registered P5 terminal gate regresses.
pub fn assert_p5_dod_terminal_gate_matrix() {
    let workspace = workspace_root();
    assert_required_gate_sources_present(&workspace);

    eprintln!(
        "p5_dod_terminal_gate_matrix event=matrix_start gate_count={}",
        TERMINAL_GATE_COMMANDS.len()
    );

    for command in TERMINAL_GATE_COMMANDS {
        run_terminal_gate_command(workspace.as_path(), command);
    }

    eprintln!(
        "p5_dod_terminal_gate_matrix event=matrix_pass gate_count={}",
        TERMINAL_GATE_COMMANDS.len()
    );
}
