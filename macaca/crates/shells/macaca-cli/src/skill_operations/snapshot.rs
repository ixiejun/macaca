//! Skill operations snapshot command handler.
//!
//! Dual-path execution:
//! 1. Live path — GET the public Web facade when `SkillCliRuntimeTarget` carries an app id.
//! 2. Diagnostic path — report structured unavailable without importing host contracts.

use macaca_proto::{MacacaResult, TraceContext};
use tracing::info;

use super::output::{print_json, print_unavailable};
use super::types::SkillCliRuntimeTarget;

/// Print a sanitized Skill operations snapshot through the live Web facade.
pub async fn execute_skill_operations_snapshot(target: SkillCliRuntimeTarget) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client.get("").await?;
        info!(
            app_id = %client.app_id,
            "CLI emitted live Skill operations snapshot from public Web API facade"
        );
        return print_json(response);
    }

    let trace = TraceContext::new("cli-skill-operations-snapshot");
    info!(
        trace_id = %trace.trace_id,
        command = "skill.operations.snapshot",
        "CLI Skill operations snapshot has no live Skill runtime target"
    );
    print_unavailable(trace, "skill.operations.snapshot")
}
