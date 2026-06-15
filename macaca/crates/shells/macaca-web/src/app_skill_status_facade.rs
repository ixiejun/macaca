//! Application-scoped Skill status Facade for Web routes.
//!
//! This module keeps application manifest resolution and Skill service snapshot
//! command construction out of HTTP handlers. It is intentionally a focused
//! shell-facing port so the host-composition crate can later provide the same
//! behavior without changing route code.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::app::AppRegistry;
use macaca_host_composition::runtime_host::{
    SkillPolicy, SkillServiceScope, SkillSnapshotServiceCommand,
};
use macaca_host_composition::SystemSkillClient;
use macaca_proto::{ApplicationId, MacacaError, MacacaResult, TraceContext};
use tokio::sync::RwLock;

use crate::skill_mcp::SkillMcpStatus;
use crate::workspace::AppWorkspace;

/// Provider-neutral route read model for one agent's visible Skill snapshot.
///
/// Skill discovery executes behind the host-owned Skill service client. HTTP
/// handlers receive only strings and flags needed for rendering, so they do not
/// construct runtime-host commands or inspect application manifests.
#[derive(Clone, Debug)]
pub struct AppSkillStatusSnapshot {
    pub agent: String,
    pub visible: Vec<AppSkillSnapshotEntry>,
    pub filtered: Vec<AppFilteredSkillSnapshotEntry>,
    pub mcp: Vec<SkillMcpStatus>,
    pub truncated: bool,
    pub compact: bool,
}

/// Visible Skill row for application-scoped route responses.
#[derive(Clone, Debug)]
pub struct AppSkillSnapshotEntry {
    pub name: String,
    pub description: String,
    pub location: String,
    pub source: String,
}

/// Filtered Skill row with the policy reason shown to operators.
#[derive(Clone, Debug)]
pub struct AppFilteredSkillSnapshotEntry {
    pub name: String,
    pub reason: String,
    pub source: String,
}

/// Focused shell-facing port for app-scoped Skill status snapshots.
///
/// Design pattern: **Facade + Adapter**. Implementations adapt application
/// manifest agent declarations into Skill service commands and return bounded
/// route-safe rows.
#[async_trait]
pub trait SystemAppSkillStatusClient: Send + Sync {
    async fn app_skill_status_snapshots(
        &self,
        app_id: ApplicationId,
        agent_filter: Option<String>,
    ) -> MacacaResult<Vec<AppSkillStatusSnapshot>>;
}

/// In-process Adapter over the composed application registry and Skill service.
pub struct WebAppSkillStatusClient {
    registry: Arc<RwLock<AppRegistry>>,
    app_workspaces: Arc<RwLock<std::collections::HashMap<ApplicationId, AppWorkspace>>>,
    skill_client: Arc<dyn SystemSkillClient>,
}

impl WebAppSkillStatusClient {
    /// Create a route-facing client from bootstrap-owned host handles.
    pub fn new(
        registry: Arc<RwLock<AppRegistry>>,
        app_workspaces: Arc<RwLock<std::collections::HashMap<ApplicationId, AppWorkspace>>>,
        skill_client: Arc<dyn SystemSkillClient>,
    ) -> Self {
        Self {
            registry,
            app_workspaces,
            skill_client,
        }
    }
}

#[async_trait]
impl SystemAppSkillStatusClient for WebAppSkillStatusClient {
    async fn app_skill_status_snapshots(
        &self,
        app_id: ApplicationId,
        agent_filter: Option<String>,
    ) -> MacacaResult<Vec<AppSkillStatusSnapshot>> {
        let app = {
            let registry = self.registry.read().await;
            registry
                .get_app(&app_id)
                .cloned()
                .ok_or_else(|| MacacaError::NotFound("App not found".into()))?
        };
        let agent_configs = macaca_host_composition::app::AppLoader::resolve_agent_configs(
            &app.manifest,
            &app.path,
        )?;
        let workspace_root = {
            let workspaces = self.app_workspaces.read().await;
            workspaces.get(&app_id).map(|ws| ws.root.clone())
        };

        let mut statuses = Vec::new();
        for agent in agent_configs {
            if agent_filter
                .as_deref()
                .is_some_and(|name| name != agent.name.as_str())
            {
                continue;
            }
            let policy = agent
                .skills
                .as_ref()
                .map(|skills| SkillPolicy {
                    allow: skills.allow.clone(),
                    deny: skills.deny.clone(),
                })
                .unwrap_or_default();
            let snapshot = self
                .skill_client
                .snapshot(SkillSnapshotServiceCommand {
                    trace: TraceContext::new("web-route-app-skills-snapshot"),
                    scope: SkillServiceScope::agent(
                        app_id,
                        "web-route-app-skills",
                        agent.name.clone(),
                    )?,
                    agent_name: agent.name.clone(),
                    workspace_dir: workspace_root.clone(),
                    app_dir: Some(app.path.clone()),
                    include_instructions: true,
                    exposure_policy: policy,
                    policy: Default::default(),
                })
                .await?;
            let mcp = crate::skill_mcp::probe_skill_mcp_servers(&snapshot).await;

            tracing::info!(
                app_id = %app_id,
                agent = %snapshot.agent,
                visible = snapshot.skills.len(),
                filtered = snapshot.filtered.len(),
                mcp_servers = mcp.len(),
                "Application Skill status snapshot prepared for shell route"
            );

            statuses.push(AppSkillStatusSnapshot {
                agent: snapshot.agent,
                visible: snapshot
                    .skills
                    .into_iter()
                    .map(|skill| AppSkillSnapshotEntry {
                        name: skill.name,
                        description: skill.description,
                        location: skill.location.display().to_string(),
                        source: skill.source,
                    })
                    .collect(),
                filtered: snapshot
                    .filtered
                    .into_iter()
                    .map(|skill| AppFilteredSkillSnapshotEntry {
                        name: skill.name,
                        reason: skill.reason,
                        source: skill.source,
                    })
                    .collect(),
                mcp,
                truncated: snapshot.truncated,
                compact: snapshot.compact,
            });
        }

        Ok(statuses)
    }
}
