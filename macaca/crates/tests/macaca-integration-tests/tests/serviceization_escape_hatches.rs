//! Static serviceization escape-hatch gate.
//!
//! This integration test is an executable specification for terminal
//! serviceization. It prevents production Rust code from growing direct
//! runtime/provider access outside service clients.

#[path = "serviceization_escape_hatches/assertions.rs"]
mod assertions;
#[path = "serviceization_escape_hatches/autonomy_schedule.rs"]
mod autonomy_schedule;
#[path = "serviceization_escape_hatches/scanner.rs"]
mod scanner;
#[path = "serviceization_escape_hatches/support.rs"]
mod support;
#[path = "serviceization_escape_hatches/terminal_debt_baseline.rs"]
mod terminal_debt_baseline;
#[path = "serviceization_escape_hatches/tokens.rs"]
mod tokens;

use assertions::{
    assert_production_literal_tokens_absent_outside_allowed_paths,
    assert_production_paths_literal_tokens_absent,
    assert_retired_escape_hatch_family_absent_in_production,
    assert_retired_escape_hatch_tokens_absent_in_production,
};
use scanner::{
    collect_production_violations, render_violations, violation_fingerprint, ScanOptions,
};

#[test]
fn serviceization_escape_hatches_reject_new_production_references() {
    let violations = collect_production_violations(ScanOptions {
        honor_terminal_exception_surfaces: true,
    });

    assert!(
        violations.is_empty(),
        "Serviceization escape-hatch freeze violations were found:{}",
        render_violations(&violations)
    );
}

/// Ignored helper — run with `cargo test -p macaca-integration-tests dump_escape_hatch_raw_fingerprints -- --ignored --nocapture`
/// when `terminal_debt_baseline.rs` must be regenerated after deliberate surface retirement.
#[test]
#[ignore = "baseline regeneration helper only"]
fn dump_escape_hatch_raw_fingerprints() {
    let violations = collect_production_violations(ScanOptions {
        honor_terminal_exception_surfaces: false,
    });
    eprintln!(
        "serviceization_escape_hatches event=dump_raw_inventory count={}",
        violations.len()
    );
    for violation in &violations {
        eprintln!("{}", violation_fingerprint(violation));
    }
}

#[test]
fn serviceization_escape_hatches_terminal_debt_inventory_matches_baseline() {
    let violations = collect_production_violations(ScanOptions {
        honor_terminal_exception_surfaces: false,
    });

    assert_eq!(
        violations.len(),
        terminal_debt_baseline::EXPECTED_RAW_VIOLATION_COUNT,
        "Raw escape-hatch violation count changed (honor_terminal_exception_surfaces=false). \
         Update terminal_debt_baseline.rs after OpenSpec-approved surface retirement.{}",
        render_violations(&violations)
    );

    let mut observed_by_family: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for violation in &violations {
        *observed_by_family.entry(violation.family).or_insert(0) += 1;
    }
    for (family, expected_count) in terminal_debt_baseline::EXPECTED_RAW_VIOLATION_BY_FAMILY {
        let observed = observed_by_family.get(family).copied().unwrap_or(0);
        assert_eq!(
            observed, *expected_count,
            "Raw escape-hatch family debt changed for {family}: observed={observed} expected={expected_count}. \
             Update terminal_debt_baseline.rs after OpenSpec-approved retirement."
        );
    }
}

#[test]
fn serviceization_escape_hatches_reconciliation_markers_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("multi-path-coordination-patch");
}

#[test]
fn serviceization_escape_hatches_autonomy_loop_boundary_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("autonomy-loop-boundary");
}

#[test]
fn serviceization_escape_hatches_web_direct_runtime_field_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("web-direct-runtime-field");
}

#[test]
fn serviceization_escape_hatches_direct_runtime_catalog_read_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("direct-runtime-catalog-read");
}

#[test]
fn serviceization_escape_hatches_provider_bridge_construction_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production(concat!(
        "provider-",
        "com",
        "pat",
        "-construction"
    ));
}

#[test]
fn serviceization_escape_hatches_local_autonomy_providers_absent_in_production() {
    assert_retired_escape_hatch_tokens_absent_in_production(&[
        "LocalSchedulerProvider",
        "LocalHeartbeatProvider",
    ]);
}

#[test]
fn serviceization_escape_hatches_autonomy_host_adapters_absent_in_production() {
    assert_retired_escape_hatch_tokens_absent_in_production(&[
        "SchedulerSystemServiceProvider",
        "HeartbeatSystemServiceProvider",
    ]);
}

#[test]
fn serviceization_escape_hatches_autonomy_supervisor_absent_in_production() {
    assert_retired_escape_hatch_tokens_absent_in_production(&["AutonomySupervisor"]);
}

#[test]
fn serviceization_escape_hatches_autonomy_service_id_literals_absent_outside_proto() {
    assert_production_literal_tokens_absent_outside_allowed_paths(
        &["\"service.scheduler\"", "\"service.heartbeat\""],
        &["crates/foundation/macaca-proto/src/"],
    );
}

#[test]
fn serviceization_escape_hatches_autonomy_service_boundary_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("autonomy-service-boundary");
}

#[test]
fn serviceization_escape_hatches_web_framework_runner_coordinator_literal_absent() {
    assert_production_paths_literal_tokens_absent(
        &[
            "crates/shells/macaca-web/src/chat_orchestrator/route_chat_v2.rs",
            "crates/shells/macaca-web/src/framework_runner/build_mode.rs",
            "crates/shells/macaca-web/src/framework_runner/sse_emitter_adapter.rs",
        ],
        &["\"coordinator\""],
    );
}

#[test]
fn serviceization_escape_hatches_hardcoded_agent_role_terminal_literals_absent() {
    assert_production_paths_literal_tokens_absent(
        &[
            "crates/application/macaca-app/src/workflow.rs",
            "crates/foundation/macaca-proto/src/agent_execution_service/mod.rs",
            "crates/foundation/macaca-proto/src/agent_execution_service/autonomous_envelope.rs",
            "crates/foundation/macaca-proto/src/agent_execution_service/command_adapters.rs",
        ],
        &[
            "\"coordinator\"",
            "\"planner\"",
            "\"worker\"",
            "\"backend\"",
            "\"frontend\"",
            "\"architect\"",
        ],
    );
}

#[test]
fn serviceization_escape_hatches_provider_model_routing_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("provider-model-routing-name");
}

#[test]
fn serviceization_escape_hatches_memory_embedding_provider_literals_absent() {
    assert_production_paths_literal_tokens_absent(
        &["crates/services/macaca-memory/src/backend.rs"],
        &["\"dashscope\""],
    );
}

#[test]
fn serviceization_escape_hatches_framework_model_impls_provider_literals_absent() {
    assert_production_paths_literal_tokens_absent(
        &[
            "crates/runtime/macaca-framework/src/model_impls.rs",
            "crates/runtime/macaca-framework/src/model_impls/openai.rs",
            "crates/runtime/macaca-framework/src/model_impls/anthropic.rs",
        ],
        &["\"openai\"", "\"anthropic\""],
    );
}

#[test]
fn serviceization_escape_hatches_os_test_fixture_role_literals_absent() {
    assert_production_paths_literal_tokens_absent(
        &[
            "crates/services/macaca-tools/src/todo/create_todo.rs",
            "crates/services/macaca-tools/src/todo/tests.rs",
            "crates/services/macaca-scheduled-agent-task/src/local_provider/tests.rs",
            "crates/foundation/macaca-persist/src/event_log_tests.rs",
            "crates/shells/macaca-web/src/session/tests.rs",
            "crates/shells/macaca-web/src/routes/tests.rs",
            "crates/shells/macaca-web/src/skill_self_evolution_observer/tests.rs",
            "crates/shells/macaca-web/src/skill_mcp/tests.rs",
        ],
        &[
            "\"coordinator\"",
            "\"planner\"",
            "\"worker\"",
            "\"backend\"",
            "\"frontend\"",
            "\"architect\"",
        ],
    );
}

#[test]
fn autonomy_schedule_management_uses_serviceized_paths_only() {
    autonomy_schedule::assert_autonomy_schedule_management_uses_serviceized_paths_only();
}
