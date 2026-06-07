//! Per-agent skill allow/deny policy resolution from application manifests.

use std::sync::Arc;
use macaca_proto::ApplicationId;
use macaca_sdk::skill::SkillPolicy;
use crate::state::AppState;
pub(crate) async fn resolve_agent_skill_policy(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> SkillPolicy {
    let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
    let Some(app) = registry.get_app(app_id) else {
        return SkillPolicy::default();
    };
    for source in &app.manifest.agents {
        let macaca_app::model::AgentSource::Inline(inline) = source else {
            continue;
        };
        if inline.name != agent_name {
            continue;
        }
        let Some(skills) = &inline.skills else {
            return SkillPolicy::default();
        };
        return SkillPolicy {
            allow: skills.allow.clone(),
            deny: skills.deny.clone(),
        };
    }
    SkillPolicy::default()
}
