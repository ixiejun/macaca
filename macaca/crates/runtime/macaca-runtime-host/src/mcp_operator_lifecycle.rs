//! Runtime-host implementation of operator-grade MCP lifecycle commands.
//!
//! The module is intentionally separate from the core MCP runtime so reload,
//! OAuth, resources, diagnostics, and exposure refresh do not make that
//! protocol module larger.  It is a thin service-owned Facade over
//! `McpRuntimeFacade` plus a small in-memory memento for auth and exposure
//! generation.  Persistent backing can replace this state later without
//! changing the provider-neutral command DTOs.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    sanitize as sanitize_mcp_operator_text, McpDiagnosticsSnapshot, McpOAuthState,
    McpOAuthStatusResult, McpOperatorServerStatus, McpReloadResult, McpResourceDescriptor,
    McpResourceListResult, McpResourceReadResult, McpResourceTemplateDescriptor,
    McpResourceTemplateListResult, McpRuntimeStatusView, McpServicePolicyHints, McpServiceScope,
    McpWatchedChange, MCP_SERVICE_ID,
};
use tokio::sync::RwLock;

use crate::mcp_runtime::{McpRuntimeFacade, McpToolPolicy};

/// Mutable operator memento shared by MCP service commands.
#[derive(Debug, Default)]
pub(crate) struct McpOperatorState {
    inner: RwLock<McpOperatorStateInner>,
}

#[derive(Debug, Default)]
struct McpOperatorStateInner {
    exposure_generation: u64,
    pending_threads: BTreeSet<String>,
    auth: BTreeMap<String, AuthRecord>,
}

#[derive(Debug, Clone)]
struct AuthRecord {
    state: McpOAuthState,
    login_flow_ref: Option<String>,
    sanitized_message: Option<String>,
}

/// Service-owned operator facade used by `McpSystemServiceProvider`.
#[derive(Debug, Clone)]
pub(crate) struct McpOperatorLifecycle {
    runtime: Arc<McpRuntimeFacade>,
    state: Arc<McpOperatorState>,
}

impl McpOperatorLifecycle {
    pub fn new(runtime: Arc<McpRuntimeFacade>, state: Arc<McpOperatorState>) -> Self {
        Self { runtime, state }
    }

    /// Register changed definitions and mark current scope for deferred
    /// exposure refresh.  The next active turn calls `refresh_exposure` and
    /// consumes the pending marker, which keeps reload side effects separate
    /// from application turn execution.
    pub async fn reload(
        &self,
        scope: &McpServiceScope,
        definitions: Vec<serde_json::Value>,
    ) -> Result<McpReloadResult, String> {
        let mut changes = Vec::new();
        let count = definitions.len();
        for value in definitions {
            let definition: crate::McpServerDefinition =
                serde_json::from_value(value).map_err(|err| err.to_string())?;
            changes.push(McpWatchedChange {
                server_id: definition.id.clone(),
                change_kind: "definition_reloaded".into(),
                sanitized_summary: "MCP server definition was reloaded".into(),
            });
            self.runtime.upsert_definition(definition).await;
        }
        let mut state = self.state.inner.write().await;
        state.exposure_generation += 1;
        if let Some(thread_id) = scope.session_id.as_ref().filter(|value| !value.is_empty()) {
            state.pending_threads.insert(thread_id.clone());
        }
        let pending_thread_ids = state.pending_threads.iter().cloned().collect();
        Ok(McpReloadResult {
            reloaded: count,
            exposure_generation: state.exposure_generation,
            pending_thread_ids,
            watched_changes: changes,
            captured_at: Utc::now(),
        })
    }

    pub async fn refresh_exposure(
        &self,
        thread_id: &str,
        policy: &McpToolPolicy,
    ) -> (bool, u64, usize) {
        let descriptors = self.runtime.tool_descriptors(policy).await;
        let visible_tool_count = descriptors.iter().filter(|item| item.is_ok()).count();
        let mut state = self.state.inner.write().await;
        let refreshed = state.pending_threads.remove(thread_id);
        (refreshed, state.exposure_generation, visible_tool_count)
    }

    pub async fn start_oauth(&self, server_id: &str) -> McpOAuthStatusResult {
        let mut state = self.state.inner.write().await;
        let flow_ref = format!("mcp-oauth-{}-{}", server_id, state.exposure_generation);
        state.auth.insert(
            server_id.to_string(),
            AuthRecord {
                state: McpOAuthState::LoginStarted,
                login_flow_ref: Some(flow_ref.clone()),
                sanitized_message: Some("OAuth login flow started".into()),
            },
        );
        status_for_auth(server_id, McpOAuthState::LoginStarted, Some(flow_ref), None)
    }

    pub async fn oauth_status(&self, server_id: &str) -> McpOAuthStatusResult {
        let state = self.state.inner.read().await;
        match state.auth.get(server_id) {
            Some(record) => status_for_auth(
                server_id,
                record.state.clone(),
                record.login_flow_ref.clone(),
                record.sanitized_message.clone(),
            ),
            None => status_for_auth(server_id, McpOAuthState::NotRequired, None, None),
        }
    }

    pub async fn status_list(
        &self,
        statuses: Vec<McpRuntimeStatusView>,
    ) -> Vec<McpOperatorServerStatus> {
        let state = self.state.inner.read().await;
        statuses
            .into_iter()
            .map(|status| {
                let auth_state = state
                    .auth
                    .get(&status.server_id)
                    .map(|record| record.state.clone())
                    .unwrap_or(McpOAuthState::NotRequired);
                McpOperatorServerStatus {
                    server_id: status.server_id,
                    state: status.state,
                    auth_state,
                    tool_count: status.exposed_tools.len(),
                    resource_count: 0,
                    resource_template_count: 0,
                    exposure_generation: state.exposure_generation,
                    failure_reason: status.failure_reason,
                }
            })
            .collect()
    }

    pub async fn resource_list(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> McpResourceListResult {
        let mut resources = Vec::new();
        for result in self.runtime.list_resources(server_id, policy).await {
            match result {
                Ok((server_id, items)) => {
                    resources.extend(items.into_iter().map(|item| McpResourceDescriptor {
                        server_id: server_id.clone(),
                        uri: item.uri,
                        name: item.name,
                        mime_type: item.mime_type,
                    }))
                }
                Err(error) => tracing::warn!(
                    error = %sanitize_mcp_operator_text(error),
                    "mcp operator skipped resource listing"
                ),
            }
        }
        McpResourceListResult {
            resources,
            captured_at: Utc::now(),
        }
    }

    pub async fn resource_template_list(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> McpResourceTemplateListResult {
        let mut templates = Vec::new();
        for result in self
            .runtime
            .list_resource_templates(server_id, policy)
            .await
        {
            match result {
                Ok((server_id, items)) => {
                    templates.extend(items.into_iter().map(|item| McpResourceTemplateDescriptor {
                        server_id: server_id.clone(),
                        uri_template: item.uri_template,
                        name: item.name,
                        mime_type: item.mime_type,
                    }))
                }
                Err(error) => tracing::warn!(
                    error = %sanitize_mcp_operator_text(error),
                    "mcp operator skipped resource template listing"
                ),
            }
        }
        McpResourceTemplateListResult {
            templates,
            captured_at: Utc::now(),
        }
    }

    pub async fn read_resource(
        &self,
        server_id: &str,
        uri: &str,
        max_bytes: usize,
        policy: &McpToolPolicy,
    ) -> McpResourceReadResult {
        match self.runtime.read_resource(server_id, uri, policy).await {
            Ok(resource) => bounded_resource(server_id, resource, max_bytes),
            Err(error) => McpResourceReadResult::denied(server_id, uri, error),
        }
    }

    pub async fn diagnostics(&self, statuses: Vec<McpRuntimeStatusView>) -> McpDiagnosticsSnapshot {
        let state = self.state.inner.read().await;
        let ready = statuses
            .iter()
            .filter(|status| status.state == "Ready")
            .count();
        let failed = statuses
            .iter()
            .filter(|status| status.state != "Ready" && status.state != "Disabled")
            .count();
        let auth_required = state
            .auth
            .values()
            .filter(|record| matches!(record.state, McpOAuthState::Required))
            .count();
        McpDiagnosticsSnapshot {
            service_id: MCP_SERVICE_ID.into(),
            exposure_generation: state.exposure_generation,
            server_count: statuses.len(),
            auth_required,
            ready,
            failed,
            captured_at: Utc::now(),
        }
    }
}

pub(crate) fn runtime_policy(snapshot: macaca_proto::McpToolPolicySnapshot) -> McpToolPolicy {
    McpToolPolicy {
        allow_servers: snapshot
            .allow_servers
            .map(|items| items.into_iter().collect()),
        deny_servers: snapshot.deny_servers.into_iter().collect(),
        allow_tools: snapshot
            .allow_tools
            .map(|items| items.into_iter().collect()),
        deny_tools: snapshot.deny_tools.into_iter().collect(),
    }
}

pub(crate) fn runtime_policy_from_hints(hints: McpServicePolicyHints) -> McpToolPolicy {
    runtime_policy(hints.tool_policy)
}

fn status_for_auth(
    server_id: &str,
    state: McpOAuthState,
    login_flow_ref: Option<String>,
    message: Option<String>,
) -> McpOAuthStatusResult {
    McpOAuthStatusResult {
        server_id: server_id.into(),
        state,
        login_flow_ref,
        sanitized_message: message,
        captured_at: Utc::now(),
    }
}

fn bounded_resource(
    server_id: &str,
    resource: macaca_framework::mcp::McpResourceRead,
    max_bytes: usize,
) -> McpResourceReadResult {
    let text = resource.text.map(|value| {
        let mut value = value;
        value.truncate(max_bytes);
        value
    });
    let blob_base64 = resource.blob.map(|value| {
        let mut value = value;
        value.truncate(max_bytes);
        value
    });
    McpResourceReadResult {
        server_id: server_id.into(),
        uri: resource.uri,
        mime_type: resource.mime_type,
        truncated: text.as_ref().is_some_and(|value| value.len() >= max_bytes)
            || blob_base64
                .as_ref()
                .is_some_and(|value| value.len() >= max_bytes),
        text,
        blob_base64,
        error_summary: None,
        captured_at: Utc::now(),
    }
}
