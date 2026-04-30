//! Agent OS level MCP registry and runtime glue.
//!
//! This module owns Agent OS policy, status and toolkit registration on top
//! of the protocol primitives in [`macaca_framework::mcp`]. It was promoted
//! from `macaca-web` so any Agent OS host (web, CLI, gateway) can mount MCP
//! runtimes without depending on HTTP scaffolding.
//!
//! Product-name handling (e.g. Playwright) is **not** encoded in control
//! flow here — see [`crate::compat`] for the declarative registry that maps
//! skill install specs to compatibility MCP definitions and declarative
//! concurrency-isolation policies.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use macaca_framework::mcp::{
    client_from_transport, register_mcp_tools_with_options, McpClient, McpSessionMode, McpTimeouts,
    McpToolNameConflictPolicy, McpToolRegistrationOptions, McpTransportConfig,
};
use macaca_framework::tool::Toolkit;
use macaca_proto::ApplicationId;
use macaca_skill::{SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

use crate::compat::{default_registry, CompatRegistry};

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

#[derive(Debug, Clone)]
struct RuntimeInstanceRecord {
    refs: usize,
    state: McpRuntimeStatusState,
    last_error: Option<String>,
    last_used: Instant,
}

/// Agent OS MCP runtime manager.
#[derive(Debug, Default)]
pub struct McpRuntimeManager {
    definitions: RwLock<BTreeMap<String, McpServerDefinition>>,
    instances: Mutex<BTreeMap<McpRuntimeKey, RuntimeInstanceRecord>>,
}

impl McpRuntimeManager {
    pub fn new() -> Self {
        Self {
            definitions: RwLock::new(BTreeMap::new()),
            instances: Mutex::new(BTreeMap::new()),
        }
    }

    pub async fn load_default() -> Self {
        let manager = Self::new();
        if let Some(path) = default_mcp_config_path() {
            let _ = manager.load_config_file(path).await;
        }
        manager
    }

    pub async fn load_config_file(&self, path: PathBuf) -> Result<(), String> {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| e.to_string())?;
        let config: McpRegistryConfig =
            serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        let mut definitions = self.definitions.write().await;
        for definition in config.into_definitions(McpDefinitionSource::Global)? {
            definitions.insert(definition.id.clone(), definition);
        }
        Ok(())
    }

    pub async fn upsert_definition(&self, definition: McpServerDefinition) {
        self.definitions
            .write()
            .await
            .insert(definition.id.clone(), definition);
    }

    pub async fn definitions(&self) -> Vec<McpServerDefinition> {
        self.definitions.read().await.values().cloned().collect()
    }

    pub async fn probe_statuses(&self, policy: &McpToolPolicy) -> Vec<McpRuntimeStatus> {
        let definitions = self.definitions().await;
        probe_definition_statuses(definitions, policy).await
    }

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

    pub async fn acquire_runtime_key(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpRuntimeKey {
        let key = runtime_key(definition, context);
        let mut instances = self.instances.lock().await;
        let record = instances
            .entry(key.clone())
            .or_insert(RuntimeInstanceRecord {
                refs: 0,
                state: McpRuntimeStatusState::Ready,
                last_error: None,
                last_used: Instant::now(),
            });
        record.refs += 1;
        record.last_used = Instant::now();
        key
    }

    pub async fn release_runtime_key(&self, key: &McpRuntimeKey) -> Option<McpRuntimeStatus> {
        let mut instances = self.instances.lock().await;
        let record = instances.get_mut(key)?;
        if record.refs > 1 {
            record.refs -= 1;
            return None;
        }
        let record = instances.remove(key)?;
        Some(McpRuntimeStatus {
            server_id: key.server_id.clone(),
            transport: "runtime".to_string(),
            lifecycle: key.scope.clone(),
            session_mode: McpSessionMode::Stateful,
            state: record.state,
            exposed_tools: Vec::new(),
            failure_reason: record.last_error,
        })
    }

    pub async fn cleanup_session(&self, session_id: &str) -> Vec<McpRuntimeStatus> {
        self.cleanup_matching(|key| key.session_id.as_deref() == Some(session_id))
            .await
    }

    pub async fn cleanup_app(&self, app_id: &ApplicationId) -> Vec<McpRuntimeStatus> {
        let app = app_id.0.to_string();
        self.cleanup_matching(|key| key.app_id.as_deref() == Some(app.as_str()))
            .await
    }

    pub async fn cleanup_all(&self) -> Vec<McpRuntimeStatus> {
        self.cleanup_matching(|_| true).await
    }

    pub async fn cleanup_idle(&self, ttl: Duration) -> Vec<McpRuntimeStatus> {
        let now = Instant::now();
        let mut instances = self.instances.lock().await;
        let keys: Vec<_> = instances
            .iter()
            .filter_map(|(key, record)| {
                (record.refs == 0 && now.duration_since(record.last_used) >= ttl)
                    .then_some(key.clone())
            })
            .collect();
        keys.into_iter()
            .filter_map(|key| {
                instances.remove(&key).map(|record| McpRuntimeStatus {
                    server_id: key.server_id,
                    transport: "runtime".to_string(),
                    lifecycle: key.scope,
                    session_mode: McpSessionMode::Stateful,
                    state: record.state,
                    exposed_tools: Vec::new(),
                    failure_reason: record.last_error,
                })
            })
            .collect()
    }

    async fn cleanup_matching(
        &self,
        matches: impl Fn(&McpRuntimeKey) -> bool,
    ) -> Vec<McpRuntimeStatus> {
        let mut instances = self.instances.lock().await;
        let keys: Vec<_> = instances
            .keys()
            .filter(|key| matches(key))
            .cloned()
            .collect();
        keys.into_iter()
            .filter_map(|key| {
                instances.remove(&key).map(|record| McpRuntimeStatus {
                    server_id: key.server_id,
                    transport: "runtime".to_string(),
                    lifecycle: key.scope,
                    session_mode: McpSessionMode::Stateful,
                    state: record.state,
                    exposed_tools: Vec::new(),
                    failure_reason: record.last_error,
                })
            })
            .collect()
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

impl McpServerConfigEntry {
    fn into_definition(self, id: String) -> Result<McpServerDefinition, String> {
        let transport = match self.transport.as_str() {
            "stdio" => {
                let command = self
                    .command
                    .ok_or_else(|| format!("MCP server {id} missing command"))?;
                // Apply declarative concurrency-isolation policy if either
                // authored on this entry OR discoverable from the registry
                // by command substring.
                let policy = self
                    .concurrency_isolation
                    .clone()
                    .or_else(|| default_registry().policy_for_command(&command));
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

    let mut client =
        match client_from_transport(definition.transport.clone(), McpTimeouts::default()) {
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

        let mut client =
            match client_from_transport(definition.transport.clone(), McpTimeouts::default()) {
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

        let runtime_key = self.acquire_runtime_key(definition, context).await;
        let runtime = Arc::clone(self);
        let closed_definition = definition.clone();
        let close_callback = on_closed.map(|on_closed| {
            let runtime = Arc::clone(&runtime);
            let runtime_key = runtime_key.clone();
            Arc::new(move || {
                let runtime = Arc::clone(&runtime);
                let runtime_key = runtime_key.clone();
                let closed_definition = closed_definition.clone();
                let on_closed = Arc::clone(&on_closed);
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        let status = runtime
                            .release_runtime_key(&runtime_key)
                            .await
                            .unwrap_or_else(|| {
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

/// Resolve MCP definitions declared by a visible skill snapshot, consulting
/// the process-default compatibility registry.
pub fn definitions_from_skill_snapshot(snapshot: &SkillSnapshot) -> Vec<McpServerDefinition> {
    definitions_from_skill_snapshot_with_registry(snapshot, default_registry())
}

/// Resolve MCP definitions with an explicit compatibility registry (for
/// tests and hosts that supply their own override layer).
pub fn definitions_from_skill_snapshot_with_registry(
    snapshot: &SkillSnapshot,
    registry: &CompatRegistry,
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
    registry: &CompatRegistry,
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
mod tests {
    use super::*;
    use macaca_skill::{SkillInstallSpec, SkillSnapshot, SkillSourceScope};

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
