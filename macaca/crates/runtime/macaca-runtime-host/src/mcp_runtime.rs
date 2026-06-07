//! Agent OS level MCP registry and runtime glue.
//!
//! This module owns Agent OS policy, status and toolkit registration on top
//! of the protocol primitives in [`macaca_framework::mcp`]. It was promoted
//! from `macaca-web` so any Agent OS host (web, CLI, gateway) can mount MCP
//! runtimes without depending on HTTP scaffolding.
//!
//! Product-name handling (e.g. Playwright) is **not** encoded in control
//! flow here — see [`crate::skill_mcp_mapping_registry`] for the declarative
//! registry that maps skill install specs to MCP definitions and concurrency
//! isolation policies.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use macaca_framework::mcp::{
    register_mcp_tools_with_options, McpClient, McpError, McpResourceDef, McpResourceRead,
    McpResourceTemplateDef, McpSessionMode, McpTimeouts, McpToolNameConflictPolicy,
    McpToolRegistrationOptions, McpTransportConfig,
};
use macaca_framework::tool::Toolkit;
use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocationResult,
    CapabilityToolOriginKind, CapabilityToolResourceScope, TraceContext,
    MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_DESCRIPTOR_DEFINITION_SOURCE,
    MCP_DESCRIPTOR_LIFECYCLE_SCOPE, MCP_SERVICE_ID,
};
use macaca_skill::{SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::skill_mcp_mapping_registry::{
    default_skill_mcp_mapping_registry, SkillMcpMappingRegistry,
};
use crate::lease::McpSessionLease;
use crate::mcp_descriptor_index::{McpToolDescriptorIndex, McpToolDescriptorRoute};
use crate::mcp_invocation_registry::McpInvocationSessionRegistry;
use crate::transport::{bridge_for_config, McpTransport};

/// Strategy used by runtime-host to create low-level MCP protocol clients.
///
/// Production code uses [`bridge_for_config`] so `macaca-framework::mcp`
/// remains the single protocol implementation.  Tests may inject deterministic
/// clients through this seam, which lets service-level policy, routing, audit
/// metadata, cleanup, and timeout behavior be validated without starting
/// concrete MCP servers.
type McpClientFactory =
    dyn Fn(&McpServerDefinition, McpTimeouts) -> Result<Box<dyn McpClient>, McpError> + Send + Sync;

/// Lifecycle scope for Agent OS managed MCP instances.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum McpLifecycleScope {
    Global,
    App,
    Session,
    AgentSession,
    Call,
}

impl Default for McpLifecycleScope {
    fn default() -> Self {
        Self::Session
    }
}

/// Declarative concurrency isolation policy applied to stdio transport args.
///
/// Use this instead of hard-coded product-specific branches (e.g. "ensure
/// `--isolated` when command contains `playwright`"). The policy is
/// populated from [`crate::compat`] mappings or from `McpServerConfigEntry`
/// YAML declarations.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ConcurrencyIsolationPolicy {
    /// Args that MUST be present after policy application.
    #[serde(default)]
    pub required_args: Vec<String>,
    /// If any existing arg starts with one of these prefixes, the policy is
    /// a no-op (operator already overrode isolation behavior explicitly).
    #[serde(default)]
    pub skip_if_any_arg_prefix: Vec<String>,
}

/// Apply a declarative concurrency-isolation policy to a list of args.
///
/// This is a pure function so it can be unit-tested and reused wherever
/// stdio args are assembled.
pub fn apply_concurrency_isolation(
    policy: &ConcurrencyIsolationPolicy,
    mut args: Vec<String>,
) -> Vec<String> {
    let already_covered = policy
        .skip_if_any_arg_prefix
        .iter()
        .any(|prefix| args.iter().any(|arg| arg.starts_with(prefix)));
    if already_covered {
        return args;
    }
    for required in &policy.required_args {
        if !args.iter().any(|arg| arg == required) {
            args.push(required.clone());
        }
    }
    args
}

/// Agent OS MCP server definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDefinition {
    pub id: String,
    #[serde(flatten)]
    pub transport: McpTransportConfig,
    #[serde(default)]
    pub lifecycle: McpLifecycleScope,
    #[serde(default = "default_session_mode")]
    pub session_mode: McpSessionMode,
    #[serde(default)]
    pub tool_prefix: Option<String>,
    #[serde(default)]
    pub required_bins: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub source: McpDefinitionSource,
    /// Declarative concurrency isolation policy — carried on the definition
    /// for audit/trace purposes. Applied at build time to stdio args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency_isolation: Option<ConcurrencyIsolationPolicy>,
}

/// Source that produced an MCP server definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpDefinitionSource {
    Global,
    App,
    Skill,
    Compatibility,
}

impl Default for McpDefinitionSource {
    fn default() -> Self {
        Self::Global
    }
}

fn default_session_mode() -> McpSessionMode {
    McpSessionMode::Stateful
}

/// YAML file model for `~/.macaca/mcp.yaml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpRegistryConfig {
    #[serde(default, rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpServerConfigEntry>,
}

impl McpRegistryConfig {
    #[deprecated(note = "Use `McpServerFactory::from_registry_config` instead.")]
    pub fn into_definitions(
        self,
        source: McpDefinitionSource,
    ) -> Result<Vec<McpServerDefinition>, String> {
        self.mcp_servers
            .into_iter()
            .map(|(id, entry)| {
                let mut definition = entry.into_definition(id)?;
                definition.source = source.clone();
                Ok(definition)
            })
            .collect()
    }
}

/// Per-server YAML config entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigEntry {
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub lifecycle: McpLifecycleScope,
    #[serde(default = "default_session_mode")]
    pub session_mode: McpSessionMode,
    #[serde(default, rename = "toolPrefix")]
    pub tool_prefix: Option<String>,
    #[serde(default, rename = "requiredBins")]
    pub required_bins: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Declarative concurrency-isolation policy authored by the operator
    /// (YAML schema: `concurrency_isolation: { required_args, skip_if_any_arg_prefix }`).
    #[serde(default, rename = "concurrencyIsolation")]
    pub concurrency_isolation: Option<ConcurrencyIsolationPolicy>,
}

fn default_true() -> bool {
    true
}

/// Runtime status state for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeStatusState {
    Ready,
    Failed,
    DependencyMissing,
    Disabled,
}

/// Redacted status returned by APIs and trace payloads.
#[derive(Debug, Clone, Serialize)]
pub struct McpRuntimeStatus {
    pub server_id: String,
    pub transport: String,
    pub lifecycle: McpLifecycleScope,
    pub session_mode: McpSessionMode,
    pub state: McpRuntimeStatusState,
    pub exposed_tools: Vec<String>,
    pub failure_reason: Option<String>,
}

/// Agent/application policy for MCP visibility.
#[derive(Debug, Clone, Default)]
pub struct McpToolPolicy {
    pub allow_servers: Option<HashSet<String>>,
    pub deny_servers: HashSet<String>,
    pub allow_tools: Option<HashSet<String>>,
    pub deny_tools: HashSet<String>,
}

impl McpToolPolicy {
    fn allows_server(&self, server_id: &str) -> bool {
        if self.deny_servers.contains(server_id) {
            return false;
        }
        self.allow_servers
            .as_ref()
            .map(|allow| allow.contains(server_id))
            .unwrap_or(true)
    }

    fn allows_tool(&self, tool_name: &str) -> bool {
        if self.deny_tools.contains(tool_name) {
            return false;
        }
        self.allow_tools
            .as_ref()
            .map(|allow| allow.contains(tool_name))
            .unwrap_or(true)
    }
}

/// Runtime context used to compute lifecycle keys and trace ownership.
#[derive(Debug, Clone, Default)]
pub struct McpRuntimeContext {
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}

impl McpRuntimeContext {
    pub fn for_agent(app_id: &ApplicationId, session_id: Option<&str>, agent_name: &str) -> Self {
        Self {
            app_id: Some(app_id.clone()),
            session_id: session_id.map(ToString::to_string),
            agent_name: Some(agent_name.to_string()),
        }
    }
}

/// Runtime instance key used for lifecycle accounting.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct McpRuntimeKey {
    pub server_id: String,
    pub scope: McpLifecycleScope,
    pub app_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}

impl McpRuntimeKey {
    /// Build the service-owned lifecycle key for an MCP server and caller scope.
    ///
    /// This method is the single keying rule used by toolkit registration,
    /// service-backed invocation, cleanup, and tests.  Keeping it centralized
    /// prevents shell adapters from accidentally creating different isolation
    /// semantics for the same MCP lifecycle declaration.
    pub(crate) fn from_definition(
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> Self {
        runtime_key(definition, context)
    }
}

/// Agent OS MCP runtime manager.
#[deprecated(note = "Use `McpRuntimeFacade` as the primary host-facing entry point.")]
pub struct McpRuntimeManager {
    definitions: RwLock<BTreeMap<String, McpServerDefinition>>,
    invocation_registry: McpInvocationSessionRegistry,
    descriptor_index: McpToolDescriptorIndex,
    client_factory: Arc<McpClientFactory>,
    timeouts: McpTimeouts,
}

#[allow(deprecated)]
impl std::fmt::Debug for McpRuntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpRuntimeManager")
            .field("definitions", &self.definitions)
            .field("invocation_registry", &self.invocation_registry)
            .field("descriptor_index", &self.descriptor_index)
            .field("timeouts", &self.timeouts)
            .finish_non_exhaustive()
    }
}

#[allow(deprecated)]
impl Default for McpRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable host-facing facade for MCP runtime orchestration.
#[allow(deprecated)]
#[derive(Debug, Clone, Default)]
pub struct McpRuntimeFacade {
    manager: Arc<McpRuntimeManager>,
}

#[allow(deprecated)]
impl McpRuntimeFacade {
    #[allow(deprecated)]
    pub fn new() -> Self {
        Self {
            manager: Arc::new(McpRuntimeManager::new()),
        }
    }

    pub fn from_manager(manager: Arc<McpRuntimeManager>) -> Self {
        Self { manager }
    }

    #[allow(deprecated)]
    pub async fn load_default() -> Self {
        Self {
            manager: Arc::new(McpRuntimeManager::load_default().await),
        }
    }

    #[allow(deprecated)]
    pub async fn upsert_definition(&self, definition: McpServerDefinition) {
        self.manager.upsert_definition(definition).await;
    }

    #[allow(deprecated)]
    pub async fn definitions(&self) -> Vec<McpServerDefinition> {
        self.manager.definitions().await
    }

    #[allow(deprecated)]
    pub async fn probe(&self, policy: &McpToolPolicy) -> Vec<McpRuntimeStatus> {
        self.manager.probe_statuses(policy).await
    }

    /// Return service-owned MCP descriptors that carry explicit routing
    /// metadata for later `mcp.tool.invoke` calls.
    ///
    /// This is the descriptor-index side of the service boundary.  Shells may
    /// render or adapt the descriptors, but the backend server/tool mapping is
    /// emitted here by the runtime-host owner instead of being reconstructed by
    /// string parsing in Web or framework code.
    #[allow(deprecated)]
    pub async fn tool_descriptors(
        &self,
        policy: &McpToolPolicy,
    ) -> Vec<Result<CapabilityToolDescriptor, String>> {
        self.manager.tool_descriptors(policy).await
    }

    /// Invoke an MCP tool through the service-owned runtime path.
    ///
    /// The method creates a scoped runtime lease before protocol side effects,
    /// dispatches to the backend MCP tool name carried by the descriptor, and
    /// releases call-scoped leases immediately after completion.  The current
    /// implementation uses the existing protocol client factory as the low-level
    /// Strategy while keeping Agent OS invocation semantics in runtime-host.
    #[allow(deprecated)]
    pub async fn invoke_tool(
        &self,
        server_id: &str,
        backend_tool_name: &str,
        visible_tool_name: &str,
        input: serde_json::Value,
        trace: TraceContext,
        context: &McpRuntimeContext,
        policy: &McpToolPolicy,
    ) -> CapabilityToolInvocationResult {
        self.manager
            .invoke_tool(
                server_id,
                backend_tool_name,
                visible_tool_name,
                input,
                trace,
                context,
                policy,
            )
            .await
    }

    /// List MCP resources through the runtime-owned protocol strategy.
    #[allow(deprecated)]
    pub async fn list_resources(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceDef>), String>> {
        self.manager.list_resources(server_id, policy).await
    }

    /// List MCP resource templates through the runtime-owned protocol strategy.
    #[allow(deprecated)]
    pub async fn list_resource_templates(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceTemplateDef>), String>> {
        self.manager
            .list_resource_templates(server_id, policy)
            .await
    }

    /// Read one MCP resource through the runtime-owned protocol strategy.
    #[allow(deprecated)]
    pub async fn read_resource(
        &self,
        server_id: &str,
        uri: &str,
        policy: &McpToolPolicy,
    ) -> Result<McpResourceRead, String> {
        self.manager.read_resource(server_id, uri, policy).await
    }

    #[allow(deprecated)]
    pub async fn register(
        &self,
        toolkit: &mut Toolkit,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
    ) -> Vec<McpRuntimeStatus> {
        Arc::clone(&self.manager)
            .register_tools(toolkit, policy, context)
            .await
    }

    #[allow(deprecated)]
    pub async fn register_definitions(
        &self,
        toolkit: &mut Toolkit,
        definitions: Vec<McpServerDefinition>,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
        on_closed: Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>>,
    ) -> Vec<McpRuntimeStatus> {
        Arc::clone(&self.manager)
            .register_definitions(toolkit, definitions, policy, context, on_closed)
            .await
    }

    #[allow(deprecated)]
    pub async fn cleanup_session(&self, session_id: &str) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_session(session_id).await
    }

    #[allow(deprecated)]
    pub async fn cleanup_app(&self, app_id: &ApplicationId) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_app(app_id).await
    }

    #[allow(deprecated)]
    pub async fn cleanup_all(&self) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_all().await
    }

    #[allow(deprecated)]
    pub async fn cleanup_idle(&self, ttl: Duration) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_idle(ttl).await
    }

    #[allow(deprecated)]
    pub async fn acquire_lease(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpSessionLease {
        self.manager.acquire_lease(definition, context).await
    }

    #[allow(deprecated)]
    pub async fn release_lease(&self, lease: McpSessionLease) -> Option<McpRuntimeStatus> {
        self.manager.release_lease(lease).await
    }
}

#[allow(deprecated)]
impl McpRuntimeManager {
    #[deprecated(note = "Use `McpRuntimeFacade::new` instead.")]
    pub fn new() -> Self {
        Self {
            definitions: RwLock::new(BTreeMap::new()),
            invocation_registry: McpInvocationSessionRegistry::default(),
            descriptor_index: McpToolDescriptorIndex::default(),
            client_factory: default_mcp_client_factory(),
            timeouts: McpTimeouts::default(),
        }
    }

    /// Build a manager with a deterministic protocol-client strategy for tests.
    ///
    /// The constructor is intentionally cfg-gated so production callers cannot
    /// bypass the standard transport bridge.  It keeps the service layer testable
    /// while preserving the microkernel boundary: runtime-host owns service
    /// policy and routing, and macaca-framework owns the concrete protocol.
    #[cfg(test)]
    fn new_with_client_factory_for_tests(
        client_factory: Arc<McpClientFactory>,
        timeouts: McpTimeouts,
    ) -> Self {
        Self {
            definitions: RwLock::new(BTreeMap::new()),
            invocation_registry: McpInvocationSessionRegistry::default(),
            descriptor_index: McpToolDescriptorIndex::default(),
            client_factory,
            timeouts,
        }
    }

    #[deprecated(note = "Use `McpRuntimeFacade::load_default` instead.")]
    pub async fn load_default() -> Self {
        let manager = Self::new();
        if let Some(path) = default_mcp_config_path() {
            let _ = manager.load_config_file(path).await;
        }
        manager
    }

    #[deprecated(note = "Use `McpServerFactory`-backed facade loading instead.")]
    pub async fn load_config_file(&self, path: PathBuf) -> Result<(), String> {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| e.to_string())?;
        let config: McpRegistryConfig =
            serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        let mut definitions = self.definitions.write().await;
        for definition in crate::factory::McpServerFactory::with_bundled_mapping_registry()
            .from_registry_config(config, McpDefinitionSource::Global)?
        {
            definitions.insert(definition.id.clone(), definition);
        }
        Ok(())
    }

    #[deprecated(note = "Use `McpRuntimeFacade::upsert_definition` instead.")]
    pub async fn upsert_definition(&self, definition: McpServerDefinition) {
        self.definitions
            .write()
            .await
            .insert(definition.id.clone(), definition);
    }

    #[deprecated(note = "Use `McpRuntimeFacade::definitions` instead.")]
    pub async fn definitions(&self) -> Vec<McpServerDefinition> {
        self.definitions.read().await.values().cloned().collect()
    }

    #[deprecated(note = "Use `McpRuntimeFacade::probe` instead.")]
    pub async fn probe_statuses(&self, policy: &McpToolPolicy) -> Vec<McpRuntimeStatus> {
        let definitions = self.definitions().await;
        probe_definition_statuses(definitions, policy).await
    }

    pub async fn tool_descriptors(
        &self,
        policy: &McpToolPolicy,
    ) -> Vec<Result<CapabilityToolDescriptor, String>> {
        let definitions = self.definitions().await;
        let mut descriptors = Vec::new();
        for definition in definitions {
            descriptors.extend(descriptors_for_definition(&definition, policy).await);
        }
        let successful = descriptors
            .iter()
            .filter_map(|descriptor| descriptor.as_ref().ok().cloned())
            .collect::<Vec<_>>();
        for outcome in self.descriptor_index.upsert_descriptors(&successful).await {
            if let Err(error) = outcome {
                tracing::warn!(
                    error = %error,
                    "mcp descriptor index rejected descriptor during catalog refresh"
                );
            }
        }
        descriptors
    }

    async fn list_resources(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceDef>), String>> {
        let definitions = self.definitions().await;
        let mut results = Vec::new();
        for definition in definitions {
            if server_id.is_some_and(|requested| requested != definition.id) {
                continue;
            }
            if let Some(error) = resource_access_error(&definition, policy) {
                results.push(Err(error));
                continue;
            }
            let server = definition.id.clone();
            results.push(match self.connected_client(&definition).await {
                Ok(mut client) => {
                    let result = client
                        .list_resources()
                        .await
                        .map(|resources| (server, resources))
                        .map_err(|error| error.to_string());
                    let _ = client.close().await;
                    result
                }
                Err(error) => Err(error),
            });
        }
        results
    }

    async fn list_resource_templates(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceTemplateDef>), String>> {
        let definitions = self.definitions().await;
        let mut results = Vec::new();
        for definition in definitions {
            if server_id.is_some_and(|requested| requested != definition.id) {
                continue;
            }
            if let Some(error) = resource_access_error(&definition, policy) {
                results.push(Err(error));
                continue;
            }
            let server = definition.id.clone();
            results.push(match self.connected_client(&definition).await {
                Ok(mut client) => {
                    let result = client
                        .list_resource_templates()
                        .await
                        .map(|templates| (server, templates))
                        .map_err(|error| error.to_string());
                    let _ = client.close().await;
                    result
                }
                Err(error) => Err(error),
            });
        }
        results
    }

    async fn read_resource(
        &self,
        server_id: &str,
        uri: &str,
        policy: &McpToolPolicy,
    ) -> Result<McpResourceRead, String> {
        let Some(definition) = self.definitions.read().await.get(server_id).cloned() else {
            return Err("unknown_mcp_server".into());
        };
        if let Some(error) = resource_access_error(&definition, policy) {
            return Err(error);
        }
        let mut client = self.connected_client(&definition).await?;
        let result = client
            .read_resource(uri)
            .await
            .map_err(|error| error.to_string());
        let _ = client.close().await;
        result
    }

    async fn connected_client(
        &self,
        definition: &McpServerDefinition,
    ) -> Result<Box<dyn McpClient>, String> {
        let mut client = self
            .create_client(definition)
            .map_err(|error| error.to_string())?;
        timeout(self.timeouts.connect, client.connect())
            .await
            .map_err(|_| "connect_timeout".to_string())?
            .map_err(|error| error.to_string())?;
        Ok(client)
    }

    async fn invoke_tool(
        &self,
        server_id: &str,
        backend_tool_name: &str,
        visible_tool_name: &str,
        input: serde_json::Value,
        trace: TraceContext,
        context: &McpRuntimeContext,
        policy: &McpToolPolicy,
    ) -> CapabilityToolInvocationResult {
        let Some(definition) = self.definitions.read().await.get(server_id).cloned() else {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                tool = visible_tool_name,
                "mcp service invocation rejected because server is unknown"
            );
            return failed_invocation_result(visible_tool_name, "unknown_mcp_server", trace);
        };
        let Some(route) = self
            .descriptor_index
            .route_for_visible_tool(visible_tool_name)
            .await
        else {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                tool = visible_tool_name,
                "mcp service invocation rejected because descriptor route is unknown"
            );
            return failed_invocation_result(
                visible_tool_name,
                "mcp_descriptor_route_unknown",
                trace,
            );
        };
        if route.lifecycle != definition.lifecycle {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                visible_tool = visible_tool_name,
                route_lifecycle = ?route.lifecycle,
                definition_lifecycle = ?definition.lifecycle,
                "mcp service invocation rejected because descriptor lifecycle drifted"
            );
            return failed_invocation_result(
                visible_tool_name,
                "mcp_descriptor_lifecycle_mismatch",
                trace,
            );
        }
        if let Some(error) =
            validate_descriptor_route(&route, server_id, backend_tool_name, visible_tool_name)
        {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                backend_tool = backend_tool_name,
                visible_tool = visible_tool_name,
                reason = %error,
                "mcp service invocation rejected by descriptor index"
            );
            return failed_invocation_result(visible_tool_name, error, trace);
        }
        if !definition.enabled || !policy.allows_server(&definition.id) {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                tool = visible_tool_name,
                "mcp service invocation denied by server policy"
            );
            return failed_invocation_result(visible_tool_name, "mcp_server_denied", trace);
        }
        if !policy.allows_tool(backend_tool_name) || !policy.allows_tool(visible_tool_name) {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                backend_tool = backend_tool_name,
                visible_tool = visible_tool_name,
                "mcp service invocation denied by tool policy"
            );
            return failed_invocation_result(visible_tool_name, "mcp_tool_denied", trace);
        }
        let expected_visible_name = prefixed_tool_name(&definition, backend_tool_name);
        if expected_visible_name != visible_tool_name {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                backend_tool = backend_tool_name,
                visible_tool = visible_tool_name,
                expected_visible_tool = %expected_visible_name,
                "mcp service invocation rejected because descriptor routing did not match"
            );
            return failed_invocation_result(visible_tool_name, "mcp_descriptor_mismatch", trace);
        }
        if let Some(missing) = missing_required_bin(&definition) {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                missing_dependency = %missing,
                "mcp service invocation unavailable because a required binary is missing"
            );
            return failed_invocation_result(
                visible_tool_name,
                format!("missing dependency: {missing}"),
                trace,
            );
        }

        let mut client = match self.create_client(&definition) {
            Ok(client) => client,
            Err(error) => {
                tracing::warn!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    error = %error,
                    "mcp service invocation failed to create protocol client"
                );
                return failed_invocation_result(visible_tool_name, error.to_string(), trace);
            }
        };

        let started = Instant::now();
        let input_hash = stable_json_hash(&input);
        let lease = self.acquire_lease(&definition, context).await;
        tracing::info!(
            trace_id = %trace.trace_id,
            server_id = %definition.id,
            backend_tool = backend_tool_name,
            visible_tool = visible_tool_name,
            lifecycle = ?definition.lifecycle,
            input_hash = %input_hash,
            "mcp service invocation dispatch started"
        );

        let outcome = async {
            timeout(self.timeouts.connect, client.connect())
                .await
                .map_err(|_| "connect_timeout".to_string())?
                .map_err(|error| error.to_string())?;
            timeout(
                self.timeouts.call_tool,
                client.call_tool(backend_tool_name, input),
            )
            .await
            .map_err(|_| "call_tool_timeout".to_string())?
            .map_err(|error| error.to_string())
        }
        .await;
        let close_result = client.close().await;

        let cleanup_reason = if close_result.is_ok() {
            "closed"
        } else {
            "close_failed"
        };
        let force_release = matches!(definition.lifecycle, McpLifecycleScope::Call);
        let _ = self
            .release_lease_with_reason(lease, force_release, cleanup_reason)
            .await;
        tracing::info!(
            trace_id = %trace.trace_id,
            server_id = %definition.id,
            lifecycle = ?definition.lifecycle,
            cleanup_status = cleanup_reason,
            "mcp service invocation cleanup recorded"
        );

        match outcome {
            Ok(result) if !result.is_error => {
                let output = serde_json::json!({
                    "content": result.content,
                    "metadata": result.metadata,
                });
                let output_hash = stable_json_hash(&output);
                tracing::info!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    backend_tool = backend_tool_name,
                    visible_tool = visible_tool_name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    output_hash = %output_hash,
                    "mcp service invocation completed"
                );
                let mut service_result = CapabilityToolInvocationResult::ok(
                    MCP_SERVICE_ID,
                    CapabilityToolOriginKind::Mcp,
                    visible_tool_name,
                    output,
                    trace,
                );
                service_result
                    .metadata
                    .insert("mcp.server_id".into(), definition.id);
                service_result.metadata.insert(
                    MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(),
                    backend_tool_name.into(),
                );
                service_result
                    .metadata
                    .insert("mcp.visible_tool_name".into(), visible_tool_name.into());
                service_result.metadata.insert(
                    "mcp.lifecycle_scope".into(),
                    lifecycle_scope_name(&definition.lifecycle).into(),
                );
                service_result
                    .metadata
                    .insert("mcp.policy_decision".into(), "allow".into());
                service_result
                    .metadata
                    .insert("mcp.reason_code".into(), "ok".into());
                service_result
                    .metadata
                    .insert("mcp.input_hash".into(), input_hash);
                service_result
                    .metadata
                    .insert("mcp.output_hash".into(), output_hash);
                service_result.metadata.insert(
                    "mcp.latency_ms".into(),
                    (started.elapsed().as_millis() as u64).to_string(),
                );
                service_result
            }
            Ok(result) => {
                let summary = serde_json::to_string(&result.content)
                    .unwrap_or_else(|_| "mcp_tool_error".to_string());
                let reason = sanitize_error(summary);
                tracing::warn!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    backend_tool = backend_tool_name,
                    visible_tool = visible_tool_name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    reason_code = %reason,
                    "mcp service invocation returned an MCP error result"
                );
                failed_invocation_result_with_metadata(
                    visible_tool_name,
                    reason,
                    trace,
                    server_id,
                    backend_tool_name,
                    &definition.lifecycle,
                    input_hash,
                    started.elapsed(),
                )
            }
            Err(error) => {
                let reason = sanitize_error(error);
                tracing::warn!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    backend_tool = backend_tool_name,
                    visible_tool = visible_tool_name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    reason_code = %reason,
                    "mcp service invocation failed"
                );
                failed_invocation_result_with_metadata(
                    visible_tool_name,
                    reason,
                    trace,
                    server_id,
                    backend_tool_name,
                    &definition.lifecycle,
                    input_hash,
                    started.elapsed(),
                )
            }
        }
    }

    #[deprecated(note = "Use `McpRuntimeFacade::register` instead.")]
    pub async fn register_tools(
        self: &Arc<Self>,
        toolkit: &mut Toolkit,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
    ) -> Vec<McpRuntimeStatus> {
        let definitions = self.definitions().await;
        self.register_definitions(toolkit, definitions, policy, context, None)
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::register_definitions` instead.")]
    pub async fn register_definitions(
        self: &Arc<Self>,
        toolkit: &mut Toolkit,
        definitions: Vec<McpServerDefinition>,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
        on_closed: Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>>,
    ) -> Vec<McpRuntimeStatus> {
        let mut statuses = Vec::new();
        for definition in definitions {
            if !definition.enabled || !policy.allows_server(&definition.id) {
                statuses.push(status_for_definition(
                    &definition,
                    McpRuntimeStatusState::Disabled,
                    Vec::new(),
                    None,
                ));
                continue;
            }
            let status = self
                .register_definition_tools(toolkit, &definition, policy, context, on_closed.clone())
                .await;
            statuses.push(status);
        }
        statuses
    }

    #[deprecated(note = "Use `acquire_lease` for explicit runtime ownership.")]
    pub async fn acquire_runtime_key(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpRuntimeKey {
        self.invocation_registry
            .acquire(definition, context)
            .await
            .into_key()
    }

    #[deprecated(note = "Use `release_lease` for explicit runtime ownership release.")]
    pub async fn release_runtime_key(&self, key: &McpRuntimeKey) -> Option<McpRuntimeStatus> {
        let lease = McpSessionLease::new(key.clone());
        self.invocation_registry
            .release(&lease, false, "released")
            .await
    }

    pub async fn acquire_lease(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpSessionLease {
        self.invocation_registry.acquire(definition, context).await
    }

    /// Create a protocol client through the configured Strategy.
    ///
    /// This wrapper keeps all call sites honest: lifecycle/policy/descriptor
    /// validation happens before this method is reached, and this method is the
    /// only place where runtime-host crosses into the low-level MCP protocol
    /// implementation for service-backed invocation.
    fn create_client(
        &self,
        definition: &McpServerDefinition,
    ) -> Result<Box<dyn McpClient>, McpError> {
        (self.client_factory)(definition, self.timeouts)
    }

    pub async fn release_lease(&self, lease: McpSessionLease) -> Option<McpRuntimeStatus> {
        self.release_lease_with_reason(lease, false, "released")
            .await
    }

    async fn release_lease_with_reason(
        &self,
        lease: McpSessionLease,
        force_remove: bool,
        reason: &str,
    ) -> Option<McpRuntimeStatus> {
        self.invocation_registry
            .release(&lease, force_remove, reason)
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_session` instead.")]
    pub async fn cleanup_session(&self, session_id: &str) -> Vec<McpRuntimeStatus> {
        self.invocation_registry
            .cleanup_matching(
                |key| key.session_id.as_deref() == Some(session_id),
                "session_cleanup",
            )
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_app` instead.")]
    pub async fn cleanup_app(&self, app_id: &ApplicationId) -> Vec<McpRuntimeStatus> {
        let app = app_id.0.to_string();
        self.invocation_registry
            .cleanup_matching(
                |key| key.app_id.as_deref() == Some(app.as_str()),
                "app_cleanup",
            )
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_all` instead.")]
    pub async fn cleanup_all(&self) -> Vec<McpRuntimeStatus> {
        self.invocation_registry
            .cleanup_matching(|_| true, "all_cleanup")
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_idle` instead.")]
    pub async fn cleanup_idle(&self, ttl: Duration) -> Vec<McpRuntimeStatus> {
        self.invocation_registry.cleanup_idle(ttl).await
    }
}

pub async fn probe_definition_statuses(
    definitions: Vec<McpServerDefinition>,
    policy: &McpToolPolicy,
) -> Vec<McpRuntimeStatus> {
    let mut statuses = Vec::new();
    for definition in definitions {
        if !definition.enabled || !policy.allows_server(&definition.id) {
            statuses.push(status_for_definition(
                &definition,
                McpRuntimeStatusState::Disabled,
                Vec::new(),
                None,
            ));
            continue;
        }
        statuses.push(probe_definition(&definition, policy).await);
    }
    statuses
}

async fn descriptors_for_definition(
    definition: &McpServerDefinition,
    policy: &McpToolPolicy,
) -> Vec<Result<CapabilityToolDescriptor, String>> {
    if !definition.enabled || !policy.allows_server(&definition.id) {
        return Vec::new();
    }
    if let Some(missing) = missing_required_bin(definition) {
        tracing::warn!(
            server_id = %definition.id,
            missing_dependency = %missing,
            "mcp descriptor index skipped unavailable server"
        );
        return vec![Err(format!("missing dependency: {missing}"))];
    }

    let transport = bridge_for_config(definition.transport.clone());
    let mut client = match transport.create_client(McpTimeouts::default()) {
        Ok(client) => client,
        Err(error) => return vec![Err(error.to_string())],
    };
    if let Err(error) = timeout(McpTimeouts::default().connect, client.connect())
        .await
        .map_err(|_| "connect_timeout".to_string())
        .and_then(|result| result.map_err(|error| error.to_string()))
    {
        let _ = client.close().await;
        return vec![Err(error)];
    }
    let tools = match timeout(McpTimeouts::default().list_tools, client.list_tools()).await {
        Ok(Ok(tools)) => tools,
        Ok(Err(error)) => {
            let _ = client.close().await;
            return vec![Err(error.to_string())];
        }
        Err(_) => {
            let _ = client.close().await;
            return vec![Err("tools_list_timeout".to_string())];
        }
    };
    let _ = client.close().await;

    tools
        .into_iter()
        .filter(|tool| policy.allows_tool(&tool.name))
        .filter_map(|tool| {
            let visible_name = prefixed_tool_name(definition, &tool.name);
            policy
                .allows_tool(&visible_name)
                .then_some((tool, visible_name))
        })
        .map(|(tool, visible_name)| descriptor_from_tool(definition, tool, visible_name))
        .collect()
}

fn descriptor_from_tool(
    definition: &McpServerDefinition,
    tool: macaca_framework::mcp::McpToolDef,
    visible_name: String,
) -> Result<CapabilityToolDescriptor, String> {
    let mut descriptor = CapabilityToolDescriptor::new(
        MCP_SERVICE_ID,
        definition.id.clone(),
        format!("mcp.tool.{}.{}", definition.id, tool.name),
        visible_name,
        tool.description,
        tool.input_schema,
        CapabilityToolOriginKind::Mcp,
    )
    .map_err(|error| error.to_string())?
    .with_display(Some(tool.name.clone()), definition.tool_prefix.clone())
    .with_policy_hints(
        vec!["mcp.tool.invoke".into()],
        vec![resource_scope_for_lifecycle(&definition.lifecycle)],
    );
    descriptor
        .metadata
        .insert(MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(), tool.name);
    descriptor.metadata.insert(
        MCP_DESCRIPTOR_LIFECYCLE_SCOPE.into(),
        lifecycle_scope_name(&definition.lifecycle).into(),
    );
    descriptor.metadata.insert(
        MCP_DESCRIPTOR_DEFINITION_SOURCE.into(),
        format!("{:?}", definition.source),
    );
    Ok(descriptor)
}

fn resource_scope_for_lifecycle(lifecycle: &McpLifecycleScope) -> CapabilityToolResourceScope {
    match lifecycle {
        McpLifecycleScope::Global => CapabilityToolResourceScope::Global,
        McpLifecycleScope::App => CapabilityToolResourceScope::Application,
        McpLifecycleScope::Session => CapabilityToolResourceScope::Session,
        McpLifecycleScope::AgentSession => CapabilityToolResourceScope::AgentSession,
        McpLifecycleScope::Call => CapabilityToolResourceScope::Call,
    }
}

fn resource_access_error(
    definition: &McpServerDefinition,
    policy: &McpToolPolicy,
) -> Option<String> {
    if !definition.enabled || !policy.allows_server(&definition.id) {
        return Some("mcp_server_denied".into());
    }
    missing_required_bin(definition).map(|missing| format!("missing dependency: {missing}"))
}

fn lifecycle_scope_name(lifecycle: &McpLifecycleScope) -> &'static str {
    match lifecycle {
        McpLifecycleScope::Global => "global",
        McpLifecycleScope::App => "app",
        McpLifecycleScope::Session => "session",
        McpLifecycleScope::AgentSession => "agent_session",
        McpLifecycleScope::Call => "call",
    }
}

impl McpServerConfigEntry {
    pub(crate) fn into_definition_with_registry(
        self,
        id: String,
        registry: &SkillMcpMappingRegistry,
    ) -> Result<McpServerDefinition, String> {
        let transport = match self.transport.as_str() {
            "stdio" => {
                let command = self
                    .command
                    .ok_or_else(|| format!("MCP server {id} missing command"))?;
                let policy = self
                    .concurrency_isolation
                    .clone()
                    .or_else(|| registry.policy_for_command(&command));
                let args = policy
                    .as_ref()
                    .map(|p| apply_concurrency_isolation(p, self.args.clone()))
                    .unwrap_or(self.args);
                McpTransportConfig::Stdio {
                    command,
                    args,
                    env: self.env,
                    cwd: self.cwd,
                }
            }
            "sse" => McpTransportConfig::Sse {
                url: self
                    .url
                    .ok_or_else(|| format!("MCP server {id} missing url"))?,
                headers: self.headers,
            },
            "streamable_http" => McpTransportConfig::StreamableHttp {
                url: self
                    .url
                    .ok_or_else(|| format!("MCP server {id} missing url"))?,
                headers: self.headers,
            },
            other => return Err(format!("Unsupported MCP transport: {other}")),
        };
        Ok(McpServerDefinition {
            id,
            transport,
            lifecycle: self.lifecycle,
            session_mode: self.session_mode,
            tool_prefix: self.tool_prefix,
            required_bins: self.required_bins,
            enabled: self.enabled,
            source: McpDefinitionSource::Global,
            concurrency_isolation: self.concurrency_isolation,
        })
    }

    fn into_definition(self, id: String) -> Result<McpServerDefinition, String> {
        self.into_definition_with_registry(id, default_skill_mcp_mapping_registry())
    }
}

fn default_mcp_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".macaca").join("mcp.yaml"))
}

async fn probe_definition(
    definition: &McpServerDefinition,
    policy: &McpToolPolicy,
) -> McpRuntimeStatus {
    if let Some(missing) = missing_required_bin(definition) {
        return status_for_definition(
            definition,
            McpRuntimeStatusState::DependencyMissing,
            Vec::new(),
            Some(format!("missing dependency: {missing}")),
        );
    }

    let transport = bridge_for_config(definition.transport.clone());
    let mut client = match transport.create_client(McpTimeouts::default()) {
        Ok(client) => client,
        Err(error) => {
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            )
        }
    };
    let connected = timeout(Duration::from_secs(15), client.connect()).await;
    if let Err(error) = flatten_timeout_result(connected) {
        return status_for_definition(
            definition,
            McpRuntimeStatusState::Failed,
            Vec::new(),
            Some(error),
        );
    }

    let tools = match timeout(Duration::from_secs(15), client.list_tools()).await {
        Ok(Ok(tools)) => tools,
        Ok(Err(error)) => {
            let _ = client.close().await;
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            );
        }
        Err(_) => {
            let _ = client.close().await;
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some("tools_list_timeout".to_string()),
            );
        }
    };
    let _ = client.close().await;

    let exposed_tools = tools
        .into_iter()
        .filter(|tool| policy.allows_tool(&tool.name))
        .map(|tool| prefixed_tool_name(definition, &tool.name))
        .filter(|tool| policy.allows_tool(tool))
        .collect();
    status_for_definition(
        definition,
        McpRuntimeStatusState::Ready,
        exposed_tools,
        None,
    )
}

#[allow(deprecated)]
impl McpRuntimeManager {
    async fn register_definition_tools(
        self: &Arc<Self>,
        toolkit: &mut Toolkit,
        definition: &McpServerDefinition,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
        on_closed: Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>>,
    ) -> McpRuntimeStatus {
        if let Some(missing) = missing_required_bin(definition) {
            return status_for_definition(
                definition,
                McpRuntimeStatusState::DependencyMissing,
                Vec::new(),
                Some(format!("missing dependency: {missing}")),
            );
        }

        let transport = bridge_for_config(definition.transport.clone());
        let mut client = match transport.create_client(McpTimeouts::default()) {
            Ok(client) => client,
            Err(error) => {
                return status_for_definition(
                    definition,
                    McpRuntimeStatusState::Failed,
                    Vec::new(),
                    Some(error.to_string()),
                )
            }
        };
        if let Err(error) = client.connect().await {
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            );
        }

        let lease = self.acquire_lease(definition, context).await;
        let runtime = Arc::clone(self);
        let closed_definition = definition.clone();
        let close_callback = on_closed.map(|on_closed| {
            let runtime = Arc::clone(&runtime);
            let lease = lease.clone();
            Arc::new(move || {
                let runtime = Arc::clone(&runtime);
                let lease = lease.clone();
                let closed_definition = closed_definition.clone();
                let on_closed = Arc::clone(&on_closed);
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        let status = runtime.release_lease(lease).await.unwrap_or_else(|| {
                            status_for_definition(
                                &closed_definition,
                                McpRuntimeStatusState::Ready,
                                Vec::new(),
                                None,
                            )
                        });
                        on_closed(status);
                    });
                }
            }) as Arc<dyn Fn() + Send + Sync>
        });

        let client: Arc<RwLock<dyn McpClient>> = Arc::new(RwLock::new(ClientBox { inner: client }));
        let options = McpToolRegistrationOptions {
            group_name: format!("mcp:{}", definition.id),
            conflict_policy: definition
                .tool_prefix
                .clone()
                .map(McpToolNameConflictPolicy::Prefix)
                .unwrap_or(McpToolNameConflictPolicy::Raise),
            disabled_tools: policy.deny_tools.clone(),
            on_close: close_callback,
        };
        let result = register_mcp_tools_with_options(toolkit, Arc::clone(&client), options).await;
        match result {
            Ok(registered_tools) => {
                let exposed_tools = registered_tools
                    .into_iter()
                    .filter(|name| policy.allows_tool(name))
                    .collect();
                status_for_definition(
                    definition,
                    McpRuntimeStatusState::Ready,
                    exposed_tools,
                    None,
                )
            }
            Err(error) => status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            ),
        }
    }
}

struct ClientBox {
    inner: Box<dyn McpClient>,
}

#[async_trait::async_trait]
impl McpClient for ClientBox {
    async fn connect(&mut self) -> Result<(), macaca_framework::mcp::McpError> {
        self.inner.connect().await
    }

    async fn list_tools(
        &mut self,
    ) -> Result<Vec<macaca_framework::mcp::McpToolDef>, macaca_framework::mcp::McpError> {
        self.inner.list_tools().await
    }

    async fn call_tool(
        &mut self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<macaca_framework::mcp::McpCallResult, macaca_framework::mcp::McpError> {
        self.inner.call_tool(name, args).await
    }

    async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
        self.inner.list_resources().await
    }

    async fn list_resource_templates(&mut self) -> Result<Vec<McpResourceTemplateDef>, McpError> {
        self.inner.list_resource_templates().await
    }

    async fn read_resource(&mut self, uri: &str) -> Result<McpResourceRead, McpError> {
        self.inner.read_resource(uri).await
    }

    async fn close(&mut self) -> Result<(), macaca_framework::mcp::McpError> {
        self.inner.close().await
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }
}

fn missing_required_bin(definition: &McpServerDefinition) -> Option<String> {
    let mut required = definition.required_bins.clone();
    if let McpTransportConfig::Stdio { command, .. } = &definition.transport {
        required.push(command.clone());
    }
    required.into_iter().find(|bin| !command_exists(bin))
}

fn command_exists(command: &str) -> bool {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return PathBuf::from(command).is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

/// Default Strategy for constructing MCP protocol clients from definitions.
///
/// The function captures no application state and performs no policy decisions.
/// It is deliberately tiny: the runtime manager validates descriptor routing,
/// lifecycle ownership, policy, and dependency availability before this factory
/// is called, while `macaca-framework::mcp` remains responsible for speaking
/// the actual protocol over stdio/SSE/streamable HTTP transports.
fn default_mcp_client_factory() -> Arc<McpClientFactory> {
    Arc::new(|definition, timeouts| {
        let transport = bridge_for_config(definition.transport.clone());
        transport.create_client(timeouts)
    })
}

fn status_for_definition(
    definition: &McpServerDefinition,
    state: McpRuntimeStatusState,
    exposed_tools: Vec<String>,
    failure_reason: Option<String>,
) -> McpRuntimeStatus {
    McpRuntimeStatus {
        server_id: definition.id.clone(),
        transport: transport_name(&definition.transport).to_string(),
        lifecycle: definition.lifecycle.clone(),
        session_mode: definition.session_mode,
        state,
        exposed_tools,
        failure_reason,
    }
}

fn transport_name(config: &McpTransportConfig) -> &'static str {
    match config {
        McpTransportConfig::Stdio { .. } => "stdio",
        McpTransportConfig::Sse { .. } => "sse",
        McpTransportConfig::StreamableHttp { .. } => "streamable_http",
    }
}

fn runtime_key(definition: &McpServerDefinition, context: &McpRuntimeContext) -> McpRuntimeKey {
    let app_id = context.app_id.as_ref().map(|id| id.0.to_string());
    let session_id = context.session_id.clone();
    let agent_name = context.agent_name.clone();
    match definition.lifecycle {
        McpLifecycleScope::Global => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::Global,
            app_id: None,
            session_id: None,
            agent_name: None,
        },
        McpLifecycleScope::App => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::App,
            app_id,
            session_id: None,
            agent_name: None,
        },
        McpLifecycleScope::Session => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::Session,
            app_id,
            session_id,
            agent_name: None,
        },
        McpLifecycleScope::AgentSession => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::AgentSession,
            app_id,
            session_id,
            agent_name,
        },
        McpLifecycleScope::Call => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::Call,
            app_id,
            session_id,
            agent_name,
        },
    }
}

fn prefixed_tool_name(definition: &McpServerDefinition, tool_name: &str) -> String {
    definition
        .tool_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}{tool_name}"))
        .unwrap_or_else(|| tool_name.to_string())
}

fn failed_invocation_result(
    visible_tool_name: &str,
    error_summary: impl Into<String>,
    trace: TraceContext,
) -> CapabilityToolInvocationResult {
    CapabilityToolInvocationResult::failed(
        MCP_SERVICE_ID,
        CapabilityToolOriginKind::Mcp,
        visible_tool_name,
        sanitize_error(error_summary),
        trace,
    )
}

fn failed_invocation_result_with_metadata(
    visible_tool_name: &str,
    error_summary: impl Into<String>,
    trace: TraceContext,
    server_id: &str,
    backend_tool_name: &str,
    lifecycle: &McpLifecycleScope,
    input_hash: String,
    elapsed: Duration,
) -> CapabilityToolInvocationResult {
    let reason = sanitize_error(error_summary);
    let mut result = failed_invocation_result(visible_tool_name, reason.clone(), trace);
    result
        .metadata
        .insert("mcp.server_id".into(), server_id.into());
    result.metadata.insert(
        MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(),
        backend_tool_name.into(),
    );
    result.metadata.insert(
        "mcp.lifecycle_scope".into(),
        lifecycle_scope_name(lifecycle).into(),
    );
    result
        .metadata
        .insert("mcp.policy_decision".into(), "allow".into());
    result.metadata.insert("mcp.reason_code".into(), reason);
    result.metadata.insert("mcp.input_hash".into(), input_hash);
    result.metadata.insert(
        "mcp.latency_ms".into(),
        (elapsed.as_millis() as u64).to_string(),
    );
    result
}

fn validate_descriptor_route(
    route: &McpToolDescriptorRoute,
    server_id: &str,
    backend_tool_name: &str,
    visible_tool_name: &str,
) -> Option<&'static str> {
    if route.server_id != server_id {
        return Some("mcp_descriptor_server_mismatch");
    }
    if route.backend_tool_name != backend_tool_name {
        return Some("mcp_descriptor_backend_tool_mismatch");
    }
    if route.visible_tool_name != visible_tool_name {
        return Some("mcp_descriptor_visible_tool_mismatch");
    }
    None
}

fn stable_json_hash(value: &serde_json::Value) -> String {
    let serialized = serde_json::to_string(value).unwrap_or_else(|_| "unserializable".into());
    stable_text_hash(&serialized)
}

fn stable_text_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn sanitize_error(error: impl Into<String>) -> String {
    let value = error.into();
    let mut sanitized = value.replace('\n', " ").replace('\r', " ");
    sanitized.truncate(240);
    sanitized
}

/// Resolve MCP definitions declared by a visible skill snapshot, consulting
/// the process-default compatibility registry.
#[deprecated(note = "Use `McpServerFactory::from_skill_snapshot` instead.")]
pub fn definitions_from_skill_snapshot(snapshot: &SkillSnapshot) -> Vec<McpServerDefinition> {
    definitions_from_skill_snapshot_with_registry(snapshot, default_skill_mcp_mapping_registry())
}

/// Resolve MCP definitions with an explicit compatibility registry (for
/// tests and hosts that supply their own override layer).
pub fn definitions_from_skill_snapshot_with_registry(
    snapshot: &SkillSnapshot,
    registry: &SkillMcpMappingRegistry,
) -> Vec<McpServerDefinition> {
    let mut definitions = Vec::new();
    let mut seen = HashSet::new();
    for skill in &snapshot.skills {
        for server in &skill.mcp_servers {
            if let Some(definition) = definition_from_skill_server(skill, server, registry) {
                if seen.insert(definition.id.clone()) {
                    definitions.push(definition);
                }
            }
        }
        if let Some(compat_entry) = registry.resolve_for_skill(skill) {
            let id = format!("skill:{}:{}", skill.name, compat_entry.id);
            if let Some(definition) = compat_entry.to_definition(id) {
                if seen.insert(definition.id.clone()) {
                    definitions.push(definition);
                }
            }
        }
    }
    definitions
}

fn definition_from_skill_server(
    skill: &SkillSnapshotEntry,
    server: &SkillMcpServerConfig,
    registry: &SkillMcpMappingRegistry,
) -> Option<McpServerDefinition> {
    if !server.transport.eq_ignore_ascii_case("stdio") {
        return None;
    }
    let id = format!("skill:{}:{}", skill.name, server.id);
    let policy = registry.policy_for_command(&server.command);
    let args = policy
        .as_ref()
        .map(|p| apply_concurrency_isolation(p, server.args.clone()))
        .unwrap_or_else(|| server.args.clone());
    Some(McpServerDefinition {
        id,
        transport: McpTransportConfig::Stdio {
            command: server.command.clone(),
            args,
            env: BTreeMap::new(),
            cwd: Some(skill.base_dir.clone()),
        },
        lifecycle: McpLifecycleScope::AgentSession,
        session_mode: McpSessionMode::Stateful,
        tool_prefix: server.tool_prefix.clone(),
        required_bins: vec![server.command.clone()],
        enabled: true,
        source: McpDefinitionSource::Skill,
        concurrency_isolation: policy,
    })
}

fn flatten_timeout_result<T>(
    result: Result<Result<T, macaca_framework::mcp::McpError>, tokio::time::error::Elapsed>,
) -> Result<T, String> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("connect_timeout".to_string()),
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use macaca_framework::mcp::{McpCallResult, McpError, McpToolDef};
    use macaca_framework::message::{ContentBlock, TextBlock};
    use macaca_skill::{SkillInstallSpec, SkillSnapshot, SkillSourceScope};
    use std::future::pending;

    fn stdio_definition(id: &str, command: &str) -> McpServerDefinition {
        McpServerDefinition {
            id: id.to_string(),
            transport: McpTransportConfig::Stdio {
                command: command.to_string(),
                args: Vec::new(),
                env: BTreeMap::new(),
                cwd: None,
            },
            lifecycle: McpLifecycleScope::AgentSession,
            session_mode: McpSessionMode::Stateful,
            tool_prefix: None,
            required_bins: vec![command.to_string()],
            enabled: true,
            source: McpDefinitionSource::Compatibility,
            concurrency_isolation: None,
        }
    }

    async fn seed_descriptor_route(
        manager: &McpRuntimeManager,
        definition: &McpServerDefinition,
        backend_tool_name: &str,
        visible_tool_name: &str,
    ) {
        let tool = macaca_framework::mcp::McpToolDef {
            name: backend_tool_name.into(),
            description: "Seeded test tool".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let descriptor = descriptor_from_tool(definition, tool, visible_tool_name.into()).unwrap();
        let outcomes = manager
            .descriptor_index
            .upsert_descriptors(&[descriptor])
            .await;
        assert!(outcomes[0].is_ok());
    }

    #[derive(Clone)]
    enum TestMcpClientBehavior {
        Success,
        ToolError,
        ConnectFailure,
        CallFailure,
        CallTimeout,
    }

    struct TestMcpClient {
        behavior: TestMcpClientBehavior,
        connected: bool,
    }

    #[async_trait::async_trait]
    impl McpClient for TestMcpClient {
        async fn connect(&mut self) -> Result<(), McpError> {
            if matches!(self.behavior, TestMcpClientBehavior::ConnectFailure) {
                return Err(McpError::Connection("fixture connect failed".into()));
            }
            self.connected = true;
            Ok(())
        }

        async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError> {
            Ok(vec![McpToolDef {
                name: "lookup".into(),
                description: "Lookup values".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }])
        }

        async fn call_tool(
            &mut self,
            name: &str,
            args: serde_json::Value,
        ) -> Result<McpCallResult, McpError> {
            match self.behavior {
                TestMcpClientBehavior::Success => Ok(McpCallResult {
                    content: vec![ContentBlock::Text(TextBlock {
                        text: format!("called {name} with {}", args["query"]),
                    })],
                    is_error: false,
                    metadata: Some(serde_json::json!({"fixture": true})),
                }),
                TestMcpClientBehavior::ToolError => Ok(McpCallResult {
                    content: vec![ContentBlock::Text(TextBlock {
                        text: "fixture tool rejected input".into(),
                    })],
                    is_error: true,
                    metadata: None,
                }),
                TestMcpClientBehavior::ConnectFailure => {
                    unreachable!("connect failure should short-circuit before call_tool")
                }
                TestMcpClientBehavior::CallFailure => {
                    Err(McpError::Execution("fixture call failed".into()))
                }
                TestMcpClientBehavior::CallTimeout => pending().await,
            }
        }

        async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
            Ok(vec![McpResourceDef {
                uri: "fixture://resource-a".into(),
                name: Some("Resource A".into()),
                mime_type: Some("text/plain".into()),
            }])
        }

        async fn list_resource_templates(
            &mut self,
        ) -> Result<Vec<McpResourceTemplateDef>, McpError> {
            Ok(vec![McpResourceTemplateDef {
                uri_template: "fixture://{id}".into(),
                name: Some("Fixture Template".into()),
                mime_type: Some("text/plain".into()),
            }])
        }

        async fn read_resource(&mut self, uri: &str) -> Result<McpResourceRead, McpError> {
            Ok(McpResourceRead {
                uri: uri.into(),
                mime_type: Some("text/plain".into()),
                text: Some("fixture resource body".into()),
                blob: None,
            })
        }

        async fn close(&mut self) -> Result<(), McpError> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }
    }

    fn manager_with_fixture_client(
        behavior: TestMcpClientBehavior,
        timeouts: McpTimeouts,
    ) -> Arc<McpRuntimeManager> {
        Arc::new(McpRuntimeManager::new_with_client_factory_for_tests(
            Arc::new(move |_definition, _timeouts| {
                Ok(Box::new(TestMcpClient {
                    behavior: behavior.clone(),
                    connected: false,
                }))
            }),
            timeouts,
        ))
    }

    #[tokio::test]
    async fn managed_resource_listing_and_read_use_mcp_protocol_client() {
        let manager =
            manager_with_fixture_client(TestMcpClientBehavior::Success, McpTimeouts::default());
        let facade = McpRuntimeFacade::from_manager(manager);
        let mut definition = stdio_definition("server-a", "fixture-mcp");
        definition.transport = McpTransportConfig::StreamableHttp {
            url: "http://127.0.0.1/mcp".into(),
            headers: BTreeMap::new(),
        };
        definition.required_bins.clear();
        facade.upsert_definition(definition).await;

        let resources = facade
            .list_resources(Some("server-a"), &McpToolPolicy::default())
            .await;
        assert_eq!(resources.len(), 1);
        assert_eq!(
            resources[0].as_ref().unwrap().1[0].uri,
            "fixture://resource-a"
        );

        let templates = facade
            .list_resource_templates(Some("server-a"), &McpToolPolicy::default())
            .await;
        assert_eq!(
            templates[0].as_ref().unwrap().1[0].uri_template,
            "fixture://{id}"
        );

        let resource = facade
            .read_resource(
                "server-a",
                "fixture://resource-a",
                &McpToolPolicy::default(),
            )
            .await
            .unwrap();
        assert_eq!(resource.text.as_deref(), Some("fixture resource body"));
    }

    #[test]
    fn parses_registry_config() {
        let config: McpRegistryConfig = serde_yaml::from_str(
            r#"
mcpServers:
  playwright:
    transport: stdio
    command: playwright-mcp
    args: ["--headless", "--isolated"]
    lifecycle: agent_session
    session_mode: stateful
    toolPrefix: browser_
"#,
        )
        .unwrap();
        let entry = config.mcp_servers.get("playwright").unwrap().clone();
        let definition = entry.into_definition("playwright".to_string()).unwrap();
        assert_eq!(definition.id, "playwright");
        assert_eq!(definition.lifecycle, McpLifecycleScope::AgentSession);
        assert_eq!(definition.tool_prefix.as_deref(), Some("browser_"));
    }

    #[test]
    fn yaml_entry_honors_authored_concurrency_isolation_policy() {
        let config: McpRegistryConfig = serde_yaml::from_str(
            r#"
mcpServers:
  custom:
    transport: stdio
    command: some-mcp-bin
    args: []
    concurrencyIsolation:
      required_args: ["--single"]
      skip_if_any_arg_prefix: ["--data-dir"]
"#,
        )
        .unwrap();
        let entry = config.mcp_servers.get("custom").unwrap().clone();
        let definition = entry.into_definition("custom".into()).unwrap();
        match definition.transport {
            McpTransportConfig::Stdio { args, .. } => {
                assert!(args.iter().any(|a| a == "--single"));
            }
            _ => panic!("expected stdio"),
        }
        assert!(definition.concurrency_isolation.is_some());
    }

    #[test]
    fn apply_concurrency_isolation_is_idempotent() {
        let policy = ConcurrencyIsolationPolicy {
            required_args: vec!["--isolated".into()],
            skip_if_any_arg_prefix: vec!["--user-data-dir".into(), "--isolated".into()],
        };
        let args = apply_concurrency_isolation(&policy, vec!["--headless".into()]);
        assert_eq!(args, vec!["--headless".to_string(), "--isolated".into()]);
        let again = apply_concurrency_isolation(&policy, args);
        assert_eq!(again, vec!["--headless".to_string(), "--isolated".into()]);
    }

    #[test]
    fn apply_concurrency_isolation_skips_when_operator_overrode() {
        let policy = ConcurrencyIsolationPolicy {
            required_args: vec!["--isolated".into()],
            skip_if_any_arg_prefix: vec!["--user-data-dir".into()],
        };
        let args =
            apply_concurrency_isolation(&policy, vec!["--user-data-dir=/tmp/profile".into()]);
        assert_eq!(args, vec!["--user-data-dir=/tmp/profile".to_string()]);
    }

    #[test]
    fn registry_config_redacts_into_app_source_definitions() {
        let config: McpRegistryConfig = serde_yaml::from_str(
            r#"
mcpServers:
  search:
    transport: streamable_http
    url: "http://127.0.0.1:9000/mcp"
    headers:
      Authorization: "Bearer secret"
"#,
        )
        .unwrap();
        let definitions = config.into_definitions(McpDefinitionSource::App).unwrap();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].source, McpDefinitionSource::App);
        assert!(matches!(
            definitions[0].transport,
            McpTransportConfig::StreamableHttp { .. }
        ));
    }

    #[test]
    fn skill_snapshot_imports_explicit_and_compat_mcp_definitions() {
        let snapshot = SkillSnapshot {
            agent: "researcher".into(),
            prompt: String::new(),
            skills: vec![SkillSnapshotEntry {
                name: "playwright-mcp".into(),
                description: "Browser".into(),
                source_location: PathBuf::from("/tmp/playwright/SKILL.md"),
                source_base_dir: PathBuf::from("/tmp/playwright"),
                location: PathBuf::from("/tmp/playwright/SKILL.md"),
                base_dir: PathBuf::from("/tmp/playwright"),
                source: "test".into(),
                source_scope: SkillSourceScope::MacacaCentral,
                primary_env: None,
                required_env: Vec::new(),
                install: vec![SkillInstallSpec {
                    kind: "npm".into(),
                    package: Some("@playwright/mcp".into()),
                    bins: vec!["playwright-mcp".into()],
                    ..Default::default()
                }],
                mcp_servers: vec![SkillMcpServerConfig {
                    id: "browser".into(),
                    command: "playwright-mcp".into(),
                    args: vec!["--headless".into()],
                    transport: "stdio".into(),
                    tool_prefix: None,
                }],
            }],
            filtered: Vec::new(),
            truncated: false,
            compact: false,
            version: 1,
        };

        let definitions = definitions_from_skill_snapshot(&snapshot);
        assert_eq!(definitions.len(), 2);
        assert!(definitions
            .iter()
            .all(|definition| definition.lifecycle == McpLifecycleScope::AgentSession));
        assert!(definitions.iter().any(|definition| matches!(
            definition.transport,
            McpTransportConfig::Stdio { ref args, .. } if args.iter().any(|arg| arg == "--isolated")
        )));
    }

    #[tokio::test]
    async fn runtime_key_reference_count_releases_on_last_owner() {
        let manager = McpRuntimeManager::new();
        let definition = stdio_definition("playwright", "playwright-mcp");
        let context = McpRuntimeContext {
            app_id: Some(ApplicationId(uuid::Uuid::nil())),
            session_id: Some("session-a".into()),
            agent_name: Some("agent-a".into()),
        };
        let key = manager.acquire_runtime_key(&definition, &context).await;
        let _ = manager.acquire_runtime_key(&definition, &context).await;

        assert!(manager.release_runtime_key(&key).await.is_none());
        assert!(manager.release_runtime_key(&key).await.is_some());
    }

    #[tokio::test]
    async fn facade_delegates_definitions_and_probe() {
        let facade = McpRuntimeFacade::new();
        let mut definition = stdio_definition("disabled", "missing-binary");
        definition.enabled = false;
        facade.upsert_definition(definition).await;

        let definitions = facade.definitions().await;
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].id, "disabled");

        let statuses = facade.probe(&McpToolPolicy::default()).await;
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].server_id, "disabled");
        assert_eq!(statuses[0].state, McpRuntimeStatusState::Disabled);
    }

    #[test]
    fn descriptor_from_tool_carries_sanitized_service_routing_metadata() {
        let mut definition = stdio_definition("server-a", "server-bin");
        definition.tool_prefix = Some("mcp_".into());
        let tool = macaca_framework::mcp::McpToolDef {
            name: "lookup".into(),
            description: "Lookup values".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };

        let descriptor = descriptor_from_tool(&definition, tool, "mcp_lookup".into()).unwrap();

        assert_eq!(descriptor.service_id, MCP_SERVICE_ID);
        assert_eq!(descriptor.provider_id, "server-a");
        assert_eq!(descriptor.tool_name, "mcp_lookup");
        assert_eq!(
            descriptor.metadata.get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME),
            Some(&"lookup".to_string())
        );
        assert_eq!(
            descriptor.metadata.get(MCP_DESCRIPTOR_LIFECYCLE_SCOPE),
            Some(&"agent_session".to_string())
        );
        assert!(
            !serde_json::to_string(&descriptor)
                .unwrap()
                .contains("server-bin"),
            "descriptors must not expose concrete commands or environment data"
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_returns_structured_failure_for_unknown_server() {
        let facade = McpRuntimeFacade::new();
        let result = facade
            .invoke_tool(
                "missing",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-test-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(result.service_id, MCP_SERVICE_ID);
        assert_eq!(result.origin_kind, CapabilityToolOriginKind::Mcp);
        assert_eq!(result.status, "failed");
        assert_eq!(result.error_summary.as_deref(), Some("unknown_mcp_server"));
    }

    #[tokio::test]
    async fn service_owned_invocation_rejects_unknown_descriptor_route() {
        let manager = Arc::new(McpRuntimeManager::new());
        manager
            .upsert_definition(stdio_definition("server-a", "server-bin"))
            .await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "mcp_lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-test-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(
            result.error_summary.as_deref(),
            Some("mcp_descriptor_route_unknown")
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_denies_tool_policy_before_client_creation() {
        let manager = Arc::new(McpRuntimeManager::new());
        let definition = stdio_definition("server-a", "sh");
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);
        let mut deny_tools = HashSet::new();
        deny_tools.insert("lookup".to_string());

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-test-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy {
                    deny_tools,
                    ..Default::default()
                },
            )
            .await;

        assert_eq!(result.error_summary.as_deref(), Some("mcp_tool_denied"));
    }

    #[tokio::test]
    async fn service_owned_invocation_reports_missing_binary_before_dispatch() {
        let manager = Arc::new(McpRuntimeManager::new());
        let definition = stdio_definition("server-a", "definitely-missing-mcp-bin");
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-test-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(
            result.error_summary.as_deref(),
            Some("missing dependency: definitely-missing-mcp-bin")
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_returns_success_with_sanitized_hash_metadata() {
        let manager =
            manager_with_fixture_client(TestMcpClientBehavior::Success, McpTimeouts::default());
        let mut definition = stdio_definition("server-a", "sh");
        definition.required_bins.clear();
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({"query": "alpha"}),
                TraceContext::new("mcp-success-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(result.status, "ok");
        assert_eq!(result.metadata.get("mcp.reason_code"), Some(&"ok".into()));
        assert!(result.metadata.contains_key("mcp.input_hash"));
        assert!(result.metadata.contains_key("mcp.output_hash"));
        assert!(
            !serde_json::to_string(&result.metadata)
                .unwrap()
                .contains("alpha"),
            "audit metadata must carry stable hashes instead of raw tool input"
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_reports_protocol_client_failures() {
        let manager = manager_with_fixture_client(
            TestMcpClientBehavior::ConnectFailure,
            McpTimeouts::default(),
        );
        let mut definition = stdio_definition("server-a", "sh");
        definition.required_bins.clear();
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-client-failure-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(result.status, "failed");
        assert_eq!(
            result.error_summary.as_deref(),
            Some("Connection error: fixture connect failed")
        );
        assert_eq!(
            result.metadata.get("mcp.reason_code"),
            Some(&"Connection error: fixture connect failed".into())
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_reports_protocol_call_failures() {
        let manager =
            manager_with_fixture_client(TestMcpClientBehavior::CallFailure, McpTimeouts::default());
        let mut definition = stdio_definition("server-a", "sh");
        definition.required_bins.clear();
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-call-failure-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(result.status, "failed");
        assert_eq!(
            result.error_summary.as_deref(),
            Some("Execution error: fixture call failed")
        );
        assert_eq!(
            result.metadata.get("mcp.reason_code"),
            Some(&"Execution error: fixture call failed".into())
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_reports_mcp_tool_error_results() {
        let manager =
            manager_with_fixture_client(TestMcpClientBehavior::ToolError, McpTimeouts::default());
        let mut definition = stdio_definition("server-a", "sh");
        definition.required_bins.clear();
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-tool-error-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(result.status, "failed");
        assert_eq!(
            result.error_summary.as_deref(),
            Some(r#"[{"type":"text","text":"fixture tool rejected input"}]"#)
        );
    }

    #[tokio::test]
    async fn service_owned_invocation_reports_call_timeout() {
        let manager = manager_with_fixture_client(
            TestMcpClientBehavior::CallTimeout,
            McpTimeouts {
                connect: Duration::from_millis(50),
                list_tools: Duration::from_millis(50),
                call_tool: Duration::from_millis(1),
            },
        );
        let mut definition = stdio_definition("server-a", "sh");
        definition.required_bins.clear();
        manager.upsert_definition(definition.clone()).await;
        seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
        let facade = McpRuntimeFacade::from_manager(manager);

        let result = facade
            .invoke_tool(
                "server-a",
                "lookup",
                "lookup",
                serde_json::json!({}),
                TraceContext::new("mcp-timeout-trace"),
                &McpRuntimeContext {
                    app_id: Some(ApplicationId(uuid::Uuid::nil())),
                    session_id: Some("session-a".into()),
                    agent_name: Some("agent-a".into()),
                },
                &McpToolPolicy::default(),
            )
            .await;

        assert_eq!(result.status, "failed");
        assert_eq!(result.error_summary.as_deref(), Some("call_tool_timeout"));
    }

    #[tokio::test]
    async fn facade_lease_release_delegates_to_runtime_manager() {
        let facade = McpRuntimeFacade::new();
        let definition = stdio_definition("playwright", "playwright-mcp");
        let context = McpRuntimeContext {
            app_id: Some(ApplicationId(uuid::Uuid::nil())),
            session_id: Some("session-a".into()),
            agent_name: Some("agent-a".into()),
        };

        let lease_a = facade.acquire_lease(&definition, &context).await;
        let lease_b = facade.acquire_lease(&definition, &context).await;

        assert!(facade.release_lease(lease_a).await.is_none());
        assert!(facade.release_lease(lease_b).await.is_some());
    }

    #[tokio::test]
    async fn cleanup_session_forces_release_for_matching_leases() {
        let manager = McpRuntimeManager::new();
        let definition = stdio_definition("playwright", "playwright-mcp");
        let context = McpRuntimeContext {
            app_id: Some(ApplicationId(uuid::Uuid::nil())),
            session_id: Some("session-a".into()),
            agent_name: Some("agent-a".into()),
        };

        let _lease_a = manager.acquire_lease(&definition, &context).await;
        let _lease_b = manager.acquire_lease(&definition, &context).await;

        let statuses = manager.cleanup_session("session-a").await;
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].server_id, "playwright");
    }

    #[test]
    fn policy_filters_servers_and_tools() {
        let mut deny_servers = HashSet::new();
        deny_servers.insert("blocked".to_string());
        let mut deny_tools = HashSet::new();
        deny_tools.insert("browser_install".to_string());
        let policy = McpToolPolicy {
            allow_servers: None,
            deny_servers,
            allow_tools: None,
            deny_tools,
        };
        assert!(!policy.allows_server("blocked"));
        assert!(policy.allows_server("playwright"));
        assert!(!policy.allows_tool("browser_install"));
        assert!(policy.allows_tool("browser_navigate"));
    }
}
