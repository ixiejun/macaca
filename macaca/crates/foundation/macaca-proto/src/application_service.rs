//! Provider-neutral Application Service DTOs for Route C S7.
//!
//! These types live in `macaca-proto` because both the SDK and runtime-host
//! must exchange Application Service commands without depending on
//! `macaca-app`.  `macaca-app` still owns application semantics; this module is
//! only the wire-safe command/result vocabulary used by service clients,
//! providers, Web adapters, and future remote transports.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationId, ApplicationLifecycleState,
    MacacaError, MacacaResult, PackageRuntimeKind, TraceContext,
};

/// Stable service id used by Application Service registration and SDK clients.
pub const APPLICATION_SERVICE_ID: &str = "service.application";

/// Command names accepted by the Application Service provider adapter.
pub const APPLICATION_DISCOVER_COMMAND: &str = "application.discover";
pub const APPLICATION_LOAD_COMMAND: &str = "application.load";
pub const APPLICATION_START_COMMAND: &str = "application.start";
pub const APPLICATION_STOP_COMMAND: &str = "application.stop";
pub const APPLICATION_REMOVE_COMMAND: &str = "application.remove";
pub const APPLICATION_STATUS_COMMAND: &str = "application.status";
pub const APPLICATION_SNAPSHOT_COMMAND: &str = "application.snapshot";
pub const APPLICATION_SESSION_START_COMMAND: &str = "application.session.start";
pub const APPLICATION_SESSION_RESUME_COMMAND: &str = "application.session.resume";
pub const APPLICATION_SESSION_STOP_COMMAND: &str = "application.session.stop";
pub const APPLICATION_HOST_DISPATCH_COMMAND: &str = "application.host.dispatch";
pub const APPLICATION_GENUI_SURFACE_COMMAND: &str = "application.genui.surface";
pub const APPLICATION_METADATA_QUERY_COMMAND: &str = "application.metadata.query";
pub const APPLICATION_AGENT_DELEGATE_COMMAND: &str = "application.agent.delegate";

/// Explicit scope for Application Service commands.
///
/// The scope is intentionally string/id based.  It can cross a local service
/// bus or future remote transport without carrying runtime handles, registry
/// references, kernel pointers, or presentation-shell state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceScope {
    pub application_id: Option<ApplicationId>,
    pub application_name: Option<String>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}

impl ApplicationServiceScope {
    /// Build application-scoped command metadata.
    pub fn application(application_id: ApplicationId) -> Self {
        Self {
            application_id: Some(application_id),
            application_name: None,
            session_id: None,
            agent_name: None,
        }
    }

    /// Build session-scoped command metadata after validating the session id.
    pub fn session(
        application_id: ApplicationId,
        session_id: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id: Some(application_id),
            application_name: None,
            session_id: Some(non_empty(
                session_id.into(),
                "application session_id is required",
            )?),
            agent_name: None,
        })
    }
}

/// Policy/admission hints interpreted by runtime decorators or app providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServicePolicyHints {
    pub required_permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Discover applications from configured application directories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationDiscoverCommand {
    pub trace: TraceContext,
    pub include_manifest_metadata: bool,
    pub policy: ApplicationServicePolicyHints,
}

impl ApplicationDiscoverCommand {
    /// Build a traced discovery command.
    pub fn new(trace: TraceContext) -> MacacaResult<Self> {
        validate_trace(&trace, "application discover command requires trace_id")?;
        Ok(Self {
            trace,
            include_manifest_metadata: false,
            policy: ApplicationServicePolicyHints::default(),
        })
    }
}

/// Load/admit one application without necessarily starting it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationLoadCommand {
    pub trace: TraceContext,
    pub manifest_path: Option<String>,
    pub package_ref: Option<String>,
    pub policy: ApplicationServicePolicyHints,
}

/// Start one application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStartCommand {
    pub trace: TraceContext,
    pub manifest_path: Option<String>,
    pub manifest: Option<serde_json::Value>,
    pub app_dir: Option<String>,
    pub policy: ApplicationServicePolicyHints,
}

/// Stop one running application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStopCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Remove one stopped application from the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRemoveCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Read status for one application or all applications when scope is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStatusCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Read a deterministic, sanitized Application Service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSnapshotCommand {
    pub trace: TraceContext,
    pub include_discovered: bool,
    pub include_running: bool,
}

/// Start a session envelope for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSessionStartCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Resume a session envelope for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSessionResumeCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Stop a session envelope for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSessionStopCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub reason: Option<String>,
}

/// Dispatch an ApplicationHost command through the service boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationHostDispatchServiceCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub host_command: ApplicationHostCommand,
}

/// Request app-scoped agent work through the Application Service.
///
/// This command is intentionally provider-neutral.  It carries only app,
/// session, agent, prompt, trace, and bounded JSON context so WASM runtimes,
/// Web, CLI, or future remote hosts can request delegated work without holding
/// `ApplicationExecutor`, Web state, Toolkit, or concrete provider handles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationAgentDelegateCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub target_agent: String,
    pub prompt: String,
    pub context: serde_json::Value,
    pub metadata: BTreeMap<String, String>,
}

/// Provider-neutral result for app-scoped agent delegation.
///
/// The result is a Memento-friendly summary rather than a raw worker runtime
/// object.  Hosts can persist or replay it alongside trace/audit events without
/// exposing agent internals, tool payload secrets, or presentation-shell state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationAgentDelegateResult {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub target_agent: String,
    pub task_id: Option<String>,
    pub success: bool,
    pub output: serde_json::Value,
    pub status: String,
    pub metadata: BTreeMap<String, String>,
}

/// Query one application-owned GenUI surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationGenUiSurfaceCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub surface_id: Option<String>,
}

/// Query sanitized application-owned metadata through Application Service.
///
/// The command uses the Command pattern so shells can ask for metadata through
/// the same traced service boundary used by lifecycle operations.  It carries
/// only scope and view-selection flags; providers must not attach raw manifest
/// bodies, raw agent configs, prompt templates, secrets, environment values, or
/// host payloads to the payload or logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetadataQueryCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub include_abilities: bool,
    pub include_policy: bool,
    pub include_overlay: bool,
    pub include_digest: bool,
}

impl ApplicationMetadataQueryCommand {
    /// Build a fail-closed application metadata query.
    pub fn application(trace: TraceContext, application_id: ApplicationId) -> MacacaResult<Self> {
        validate_trace(&trace, "application metadata query requires trace_id")?;
        Ok(Self {
            trace,
            scope: ApplicationServiceScope::application(application_id),
            include_abilities: true,
            include_policy: true,
            include_overlay: true,
            include_digest: true,
        })
    }
}

/// Sanitized agent view returned by Application Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceAgentView {
    pub name: String,
    pub capability_names: Vec<String>,
}

/// Sanitized application runtime view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceRuntimeView {
    pub runtime_kind: Option<PackageRuntimeKind>,
    pub lifecycle_state: ApplicationLifecycleState,
    pub compatibility_status: String,
    pub app_dir: Option<String>,
    pub skills_dir: Option<String>,
}

/// Sanitized application-owned UI runtime view.
///
/// This DTO is intentionally presentation-shell safe.  It carries only bounded
/// manifest declarations and host URLs that Web/Desktop shells can interpret
/// generically.  It never exposes raw host filesystem handles, secrets,
/// provider configuration, prompt text, or domain-specific service knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiRuntimeView {
    pub runtime: String,
    pub surface: ApplicationUiSurfaceView,
    pub framework: Option<String>,
    pub entry_url: String,
    pub sandbox: ApplicationUiSandboxView,
    pub bridge: ApplicationUiBridgeView,
    pub theme: ApplicationUiThemeView,
}

/// Sanitized placement metadata for an application-owned UI surface.
///
/// Shells use this strategy value to decide whether the loaded UI replaces the
/// workspace or augments the existing chat/session shell. It is intentionally a
/// small string DTO so Web and Desktop hosts share the same protocol without
/// importing application-manifest internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiSurfaceView {
    pub mode: String,
    pub chrome: String,
}

/// Bounded sandbox metadata for an application-owned UI surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiSandboxView {
    pub isolation: String,
    pub csp: String,
    pub network: String,
}

/// Declared bridge capabilities for an application-owned UI surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiBridgeView {
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

impl ApplicationUiBridgeView {
    /// Check whether a capability was explicitly declared by the application.
    ///
    /// The bridge route uses this fail-closed helper before dispatching any
    /// request to host services.  Keeping it on the DTO makes Web/Desktop
    /// shells share the same policy vocabulary without app-specific branches.
    pub fn declares(&self, capability: &str) -> bool {
        self.required.iter().any(|item| item == capability)
            || self.optional.iter().any(|item| item == capability)
    }
}

/// Theme ownership metadata for application-owned UI surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiThemeView {
    pub mode: String,
}

/// Generic command sent by an application-owned UI surface to the host bridge.
///
/// The shape is deliberately capability-oriented instead of framework,
/// application, or domain oriented.  React, Vue, Svelte, WebView, and future
/// desktop shells can all send the same envelope while policy and routing stay
/// centralized in the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationUiBridgeRequest {
    pub bridge_version: String,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
    pub trace_id: Option<String>,
    pub command_id: String,
    pub capability: String,
    pub service_id: Option<String>,
    pub operation: Option<String>,
    pub payload: serde_json::Value,
}

/// Generic bridge response returned to application-owned UI surfaces.
///
/// Results are transported as data so the host can preserve trace, policy, and
/// audit metadata without giving UI bundles direct access to runtime internals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationUiBridgeResponse {
    pub bridge_version: String,
    pub command_id: String,
    pub accepted: bool,
    pub result: ApplicationHostCommandResult,
}

/// Sanitized application view used by Web, CLI, and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceAppView {
    pub id: ApplicationId,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub entry_agent: Option<String>,
    pub agents: Vec<ApplicationServiceAgentView>,
    pub runtime: ApplicationServiceRuntimeView,
    pub ui: Option<ApplicationUiRuntimeView>,
    pub diagnostics: Vec<String>,
}

/// Session envelope tracked by Application Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceSessionView {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub entry_agent: Option<String>,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

/// Sanitized ability view returned by Application metadata queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationAbilityMetadataView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub implementation: String,
    pub is_entry: bool,
    pub activation_modes: Vec<String>,
    pub capability_names: Vec<String>,
    pub required_services: Vec<String>,
    pub permission_names: Vec<String>,
}

/// Sanitized entry metadata for shells that need routing hints.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationEntryMetadataView {
    pub agent_name: Option<String>,
    pub entry_kind: Option<String>,
    pub activation_mode: Option<String>,
}

/// Sanitized tool policy metadata.  It exposes declared names and counts only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationToolPolicyMetadataView {
    pub declared_tool_names: Vec<String>,
    pub execution_tool_count: usize,
}

/// Sanitized context policy metadata.  It reports presence, not config bodies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationContextPolicyMetadataView {
    pub context_config_present: bool,
    pub context_engine_declared: bool,
}

/// Sanitized AgentSkills metadata.  It reports policy presence and agent names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSkillPolicyMetadataView {
    pub agents_with_skill_policy: Vec<String>,
}

/// Sanitized MCP overlay metadata.  It reports overlay presence only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationMcpOverlayMetadataView {
    pub overlay_declared: bool,
    pub agents_with_overlay: Vec<String>,
}

/// Manifest digest metadata for cache keys and audit without raw manifest data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationManifestDigestView {
    pub algorithm: String,
    pub digest: String,
    pub source_format: String,
    pub ability_count: usize,
    pub agent_count: usize,
}

/// Complete sanitized metadata view for Web, CLI, Gateway, and adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationMetadataView {
    pub application: ApplicationServiceAppView,
    pub entry: ApplicationEntryMetadataView,
    pub abilities: Vec<ApplicationAbilityMetadataView>,
    pub tool_policy: ApplicationToolPolicyMetadataView,
    pub context_policy: ApplicationContextPolicyMetadataView,
    pub skill_policy: ApplicationSkillPolicyMetadataView,
    pub mcp_overlay: ApplicationMcpOverlayMetadataView,
    pub manifest_digest: Option<ApplicationManifestDigestView>,
    pub diagnostics: Vec<String>,
}

impl ApplicationServiceSessionView {
    /// Build a new session status view without capturing prompt content.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        entry_agent: Option<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            application_id,
            session_id: session_id.into(),
            entry_agent,
            status: status.into(),
            updated_at: Utc::now(),
        }
    }
}

/// Structured unavailable result used by service providers and null clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceUnavailable {
    pub service_id: String,
    pub operation: String,
    pub reason: String,
    pub trace_id: Option<String>,
    pub captured_at: DateTime<Utc>,
}

impl ApplicationServiceUnavailable {
    /// Create an auditable unavailable result.
    pub fn new(
        operation: impl Into<String>,
        reason: impl Into<String>,
        trace: Option<&TraceContext>,
    ) -> Self {
        Self {
            service_id: APPLICATION_SERVICE_ID.into(),
            operation: operation.into(),
            reason: reason.into(),
            trace_id: trace.map(|trace| trace.trace_id.clone()),
            captured_at: Utc::now(),
        }
    }
}

/// Memento snapshot for Application Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub discovered: Vec<ApplicationServiceAppView>,
    pub running: Vec<ApplicationServiceAppView>,
    pub sessions: Vec<ApplicationServiceSessionView>,
    pub diagnostics: Vec<ApplicationServiceUnavailable>,
    pub captured_at: DateTime<Utc>,
}

impl ApplicationServiceSnapshot {
    /// Build a healthy snapshot from sanitized views.
    pub fn healthy(
        discovered: Vec<ApplicationServiceAppView>,
        running: Vec<ApplicationServiceAppView>,
        sessions: Vec<ApplicationServiceSessionView>,
    ) -> Self {
        Self {
            service_id: APPLICATION_SERVICE_ID.into(),
            healthy: true,
            discovered,
            running,
            sessions,
            diagnostics: Vec::new(),
            captured_at: Utc::now(),
        }
    }

    /// Build an unavailable snapshot for hosts without an Application Service.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: APPLICATION_SERVICE_ID.into(),
            healthy: false,
            discovered: Vec::new(),
            running: Vec::new(),
            sessions: Vec::new(),
            diagnostics: vec![ApplicationServiceUnavailable::new(
                "application.snapshot",
                reason,
                None,
            )],
            captured_at: Utc::now(),
        }
    }
}

/// Result aliases keep SDK/provider signatures concise.
pub type ApplicationDiscoverResult = Vec<ApplicationServiceAppView>;
pub type ApplicationStartResult = ApplicationServiceAppView;
pub type ApplicationStatusResult = Vec<ApplicationServiceAppView>;
pub type ApplicationSessionResult = ApplicationServiceSessionView;
pub type ApplicationHostDispatchResult = ApplicationHostCommandResult;
pub type ApplicationMetadataResult = ApplicationMetadataView;

fn validate_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
