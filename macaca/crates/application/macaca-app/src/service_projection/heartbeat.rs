//! Heartbeat agent declaration projection (Specification + Value Object).
//!
//! Projects manifest-declared heartbeat agents into sanitized service views.
//! Performs bounded validation only — no profile file reads or scheduler dispatch.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationAbilityMetadataView,
    ApplicationContextPolicyMetadataView, ApplicationEntryMetadataView,
    ApplicationHeartbeatAgentView, ApplicationId, ApplicationManifestDigestView,
    ApplicationManifestV1, ApplicationMcpOverlayMetadataView, ApplicationMetadataView,
    ApplicationServiceAgentView, ApplicationServiceAppView, ApplicationServiceRuntimeView,
    ApplicationSkillPolicyMetadataView, ApplicationToolPolicyMetadataView, ApplicationUiBridgeView,
    ApplicationUiRuntimeView, ApplicationUiSandboxView, ApplicationUiSurfaceView,
    ApplicationUiThemeView, ApplicationWorkbenchMetadataView, PackageRuntimeKind,
};
use tracing::info;

use crate::consumption::app_entry_agent_name;
use crate::manifest_v1::{YamlApplicationManifestAdapter, YamlApplicationManifestProjection};
use crate::model::{AgentSource, AppManifest, AppStatus};

use super::support::{agent_name, sanitize_heartbeat_agent_metadata};

/// Project manifest-declared heartbeat agents into sanitized service views.
///
/// Application Framework owns this projection because heartbeat participation
/// is an application manifest contract. The function performs only bounded
/// validation and metadata copying: it does not scan profile files, read
/// `HEARTBEAT.md`, instantiate providers, dispatch Scheduler jobs, or execute
/// agents. Runtime-host and Agent Execution own those later service steps.
pub fn app_manifest_to_heartbeat_agent_views(
    manifest: &AppManifest,
) -> Vec<ApplicationHeartbeatAgentView> {
    let Some(heartbeat) = manifest
        .autonomy
        .as_ref()
        .and_then(|autonomy| autonomy.heartbeat.as_ref())
    else {
        return Vec::new();
    };
    if !heartbeat.enabled {
        info!(
            app_id = %manifest.id,
            "application heartbeat agent projection skipped because heartbeat is disabled"
        );
        return Vec::new();
    }

    let declared_agents: BTreeSet<String> = manifest.agents.iter().map(agent_name).collect();
    let views = heartbeat
        .agents
        .iter()
        .map(|agent| {
            let mut diagnostics = Vec::new();
            if !declared_agents.contains(&agent.name) {
                diagnostics.push("heartbeat_agent_unknown".to_string());
            }
            let native_profile_id = heartbeat_native_profile_id(manifest.id, &agent.name);
            let wake_scope_key = heartbeat_wake_scope_key(manifest.id, &agent.name);
            ApplicationHeartbeatAgentView {
                application_id: manifest.id,
                agent_name: agent.name.clone(),
                enabled: agent.enabled,
                profile_id: agent.profile_id.clone(),
                native_profile_id,
                wake_scope_key,
                fixed_interval_secs: agent
                    .cadence
                    .as_ref()
                    .and_then(|cadence| cadence.fixed_interval_secs),
                cooldown_secs: agent.gates.as_ref().and_then(|gates| gates.cooldown_secs),
                metadata: sanitize_heartbeat_agent_metadata(&agent.metadata),
                diagnostics,
            }
        })
        .collect::<Vec<_>>();

    info!(
        app_id = %manifest.id,
        declared_count = views.len(),
        valid_count = views.iter().filter(|view| view.diagnostics.is_empty()).count(),
        "projected sanitized application heartbeat agent declarations"
    );
    views
}

/// Compute the concrete native Heartbeat profile id for one manifest agent.
///
/// This helper is pure string projection owned by the Application Framework. It
/// does not register providers or inspect HEARTBEAT.md; runtime-host still owns
/// the Adapter step that turns this safe id into a Heartbeat profile.
pub fn heartbeat_native_profile_id(application_id: ApplicationId, agent_name: &str) -> String {
    format!(
        "profile.application.{}.agent.{}.heartbeat",
        application_id,
        heartbeat_agent_token(agent_name)
    )
}

/// Compute the concrete Heartbeat wake scope key for one manifest agent.
pub fn heartbeat_wake_scope_key(application_id: ApplicationId, agent_name: &str) -> String {
    format!(
        "application:{}.agent:{}.heartbeat",
        application_id,
        heartbeat_agent_token(agent_name)
    )
}

/// Convert manifest agent names into stable, URL/path-safe profile fragments.
///
/// Agent names are application-owned identifiers, but Heartbeat profile ids and
/// Web path parameters need predictable characters. The original name remains
/// in the sanitized declaration view for display and dispatch.
fn heartbeat_agent_token(agent_name: &str) -> String {
    let token = agent_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let token = token.trim_matches('_');
    if token.is_empty() {
        "agent".into()
    } else {
        token.into()
    }
}
