//! Migration-debt allowlist for OS-layer files exceeding the 500-line constitution.
//!
//! Each row records a known oversized production `src/**/*.rs` file. The executable
//! gate rejects any NEW violation while this baseline converges toward zero rows.
//! Sync removals with OpenSpec task §4.5.1 / §6.1.6 when splitting modules.

use super::gate::FileSizeAllowlistEntry;

/// Returns the baseline oversized-file snapshot (generated 2026-06-07, iteration 29).
pub fn entries() -> Vec<FileSizeAllowlistEntry> {
    vec![
        FileSizeAllowlistEntry::new("crates/application/macaca-app/src/manifest_v1/yaml_adapter.rs", 614, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/application/macaca-app/src/model.rs", 764, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/application/macaca-app/src/service_projection.rs", 847, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/application/macaca-app/src/workflow.rs", 625, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/facade/macaca-sdk/src/mcp_client.rs", 578, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/facade/macaca-sdk/src/skill_client.rs", 699, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/facade/macaca-sdk/src/system_facade.rs", 639, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/foundation/macaca-proto/src/application_service.rs", 684, "foundation-proto", "P5"),
        FileSizeAllowlistEntry::new("crates/foundation/macaca-proto/src/config.rs", 1573, "foundation-proto", "P5"),
        FileSizeAllowlistEntry::new("crates/foundation/macaca-proto/src/types.rs", 1514, "foundation-proto", "P5"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/a2a.rs", 873, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/agent.rs", 675, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/formatter.rs", 1354, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/mcp.rs", 1750, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/memory.rs", 1303, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/message.rs", 631, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/model_impls.rs", 893, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/pipeline.rs", 720, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/plan.rs", 934, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/react_agent.rs", 1040, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/tool.rs", 1042, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime/src/agentic_loop.rs", 1266, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs", 931, "P4-runtime-host", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime-host/src/autonomy_service_provider.rs", 745, "P4-runtime-host", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_agent_dispatch.rs", 1022, "P4-runtime-host", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime-host/src/executor/app_executor.rs", 1401, "P4-runtime-host", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime-host/src/executor/fork_manager.rs", 931, "P4-runtime-host", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-runtime-host/src/mcp_runtime.rs", 2575, "P4-runtime-host", "P4"),
        FileSizeAllowlistEntry::new("crates/services/macaca-context/src/engine.rs", 972, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-scheduled-agent-task/src/local_provider.rs", 1022, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-scheduler/src/local_provider.rs", 869, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-skill/src/agent_skill.rs", 676, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-skill/src/runtime.rs", 692, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-task/src/plan_loop.rs", 687, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-task/src/runtime.rs", 1144, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-task/src/todo_board.rs", 1155, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/services/macaca-tools/src/todo.rs", 1244, "OS-layer-debt", "P5"),
        FileSizeAllowlistEntry::new("crates/tests/macaca-integration-tests/src/pipeline_dry_run.rs", 711, "OS-layer-debt", "P5"),
    ]
}
