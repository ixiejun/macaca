//! Approved terminal surfaces for P5 terminal audit gates.
//!
//! Mirrors the terminal serviceization gate for the token families exercised by
//! P5 gates. File-level exemptions are narrow test-only surfaces; new modules
//! cannot inherit directory-wide tolerance.

use super::tokens::ForbiddenToken;

/// Returns true when `relative` may contain `token` in a terminal exception surface.
pub fn is_approved_terminal_exception_surface(relative: &str, token: &ForbiddenToken) -> bool {
    if relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("tests.rs")
    {
        return true;
    }

    match token.family {
        // These retired families have no production surfaces remaining.
        "shell-semantic-execution-owner" => {
            // Approved test surfaces may mention shell execution-owner tokens while
            // exercising terminal assertions.
            relative == "crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs"
                || relative.contains("/agent_execution_backend/tests/")
                || relative == "crates/shells/macaca-web/src/unified_agent_execution_provider_tests.rs"
                || relative == "crates/shells/macaca-web/src/unified_audit_replay_convergence_tests.rs"
                || relative.starts_with("crates/shells/macaca-web/src/framework_runner/traced_builders.rs")
                || relative == "crates/shells/macaca-web/src/loop_manager/tests.rs"
        }
        "hardcoded-agent-role" => matches!(
            relative,
            "crates/application/macaca-app/src/consumption.rs"
                | "crates/application/macaca-app/src/service_projection.rs"
                | "crates/application/macaca-app/src/workflow.rs"
                | "crates/foundation/macaca-proto/src/agent_execution_service/mod.rs"
                | "crates/foundation/macaca-proto/src/agent_execution_service/autonomous_envelope.rs"
                | "crates/foundation/macaca-proto/src/agent_execution_service/command_adapters.rs"
                | "crates/foundation/macaca-proto/src/orchestration.rs"
                | "crates/foundation/macaca-proto/src/types/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/app_executor/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/bus.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/callback.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/event_factory.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/fork_manager/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/queue.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/router.rs"
                | "crates/kernel/macaca-kernel/src/orchestrator.rs"
                | "crates/runtime/macaca-framework/src/construction.rs"
                | "crates/runtime/macaca-runtime-host/src/agent_context_service_provider.rs"
                | "crates/runtime/macaca-runtime-host/src/agent_execution_service_provider.rs"
                | "crates/services/macaca-memory/src/core/tests.rs"
                | "crates/services/macaca-task/src/claim_diagnostics.rs"
                | "crates/services/macaca-task/src/decompose.rs"
                | "crates/services/macaca-task/src/dependency.rs"
                | "crates/services/macaca-task/src/lifecycle.rs"
                | "crates/services/macaca-task/src/plan_loop/mod.rs"
                | "crates/services/macaca-task/src/scheduler.rs"
                | "crates/services/macaca-task/src/todo_board/tests.rs"
                | "crates/services/macaca-task/src/todo_store.rs"
                | "crates/services/macaca-tools/src/todo/tests.rs"
                | "crates/shells/macaca-web/src/capability_catalog.rs"
                | "crates/shells/macaca-web/src/chat_orchestrator/route_chat_v2.rs"
                | "crates/shells/macaca-web/src/framework_runner/build_mode.rs"
                | "crates/shells/macaca-web/src/framework_runner/sse_emitter_adapter.rs"
                | "crates/shells/macaca-web/src/framework_toolkit/mod.rs"
                | "crates/shells/macaca-web/src/framework_toolkit/builder.rs"
                | "crates/shells/macaca-web/src/loop_manager/mod.rs"
                | "crates/shells/macaca-web/src/orchestration_tools.rs"
                | "crates/shells/macaca-web/src/session/mod.rs"
                | "crates/shells/macaca-web/src/workspace.rs"
                | "crates/shells/macaca-web/src/workspace_knowledge_digest_capability.rs"
        ),
        "provider-model-routing-name" => {
            !(relative.starts_with("crates/kernel/")
                || relative.starts_with("crates/shells/macaca-web/")
                || relative.starts_with("crates/shells/macaca-cli/"))
                || relative.starts_with("crates/services/macaca-llm/src/")
        }
        _ => false,
    }
}
