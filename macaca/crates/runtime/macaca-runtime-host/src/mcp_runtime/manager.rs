//! MCP runtime manager — catalog, probe, resource, and descriptor operations.
//!
//! Owns the in-memory definition catalog, descriptor index, and invocation registry.
//! Protocol clients are created through the injected Strategy factory in `client.rs`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use macaca_framework::mcp::{
    McpClient, McpResourceDef, McpResourceRead, McpResourceTemplateDef, McpTimeouts,
};
use macaca_proto::CapabilityToolDescriptor;
use tokio::sync::RwLock;
use tokio::time::timeout;

use super::client::{default_mcp_client_factory, McpClientFactory};
use super::config_entry::default_mcp_config_path;
use super::descriptors::descriptors_for_definition;
use super::helpers::resource_access_error;
use super::probe::probe_definition_statuses;
use super::types::{
    McpDefinitionSource, McpRegistryConfig, McpRuntimeStatus, McpServerDefinition, McpToolPolicy,
};
use crate::mcp_descriptor_index::McpToolDescriptorIndex;
use crate::mcp_invocation_registry::McpInvocationSessionRegistry;

/// Internal Agent OS MCP runtime state owner.
///
/// `McpRuntimeFacade` is the only public host-facing entry point. The manager is
/// crate-private so service providers can share one runtime state object without
/// exposing implementation ownership to SDKs, shells, or applications.
pub(crate) struct McpRuntimeManager {
    pub(crate) definitions: RwLock<BTreeMap<String, McpServerDefinition>>,
    pub(crate) invocation_registry: McpInvocationSessionRegistry,
    pub(crate) descriptor_index: McpToolDescriptorIndex,
    pub(crate) client_factory: Arc<McpClientFactory>,
    pub(crate) timeouts: McpTimeouts,
}

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

impl Default for McpRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpRuntimeManager {
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
    pub(crate) fn new_with_client_factory_for_tests(
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
        for definition in crate::factory::McpServerFactory::with_bundled_mapping_registry()
            .from_registry_config(config, McpDefinitionSource::Global)?
        {
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

    /// Snapshot the in-memory MCP server definition catalog owned by runtime-host.
    ///
    /// Service providers call this when assembling `mcp.snapshot` responses. The
    /// catalog is a point-in-time view of registered definitions, not a live probe of
    /// remote MCP server health (use `probe_statuses` for connectivity diagnostics).
    pub async fn snapshot_server_definitions(&self) -> Vec<McpServerDefinition> {
        let definitions = self.definitions.read().await;
        tracing::trace!(
            definition_count = definitions.len(),
            "mcp manager emitting server definition snapshot"
        );
        definitions.values().cloned().collect()
    }

    pub async fn probe_statuses(&self, policy: &McpToolPolicy) -> Vec<McpRuntimeStatus> {
        let definitions = self.snapshot_server_definitions().await;
        probe_definition_statuses(definitions, policy).await
    }

    pub async fn tool_descriptors(
        &self,
        policy: &McpToolPolicy,
    ) -> Vec<Result<CapabilityToolDescriptor, String>> {
        let definitions = self.snapshot_server_definitions().await;
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

    pub(crate) async fn list_resources(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceDef>), String>> {
        let definitions = self.snapshot_server_definitions().await;
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

    pub(crate) async fn list_resource_templates(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceTemplateDef>), String>> {
        let definitions = self.snapshot_server_definitions().await;
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

    pub(crate) async fn read_resource(
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

    /// Connect a protocol client with the manager's configured connect timeout.
    pub(crate) async fn connected_client(
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
}
