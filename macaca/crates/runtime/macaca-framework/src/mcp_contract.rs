// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Provider-neutral MCP registry and transport contracts.
//!
//! This module describes MCP clients and server entries without constructing
//! concrete transports. Runtime-host remains responsible for stdio processes,
//! HTTP clients, SSE streams, streamable HTTP, policy decorators, and plugin
//! replacement. The framework receives a stable descriptor/registry contract
//! that can be mocked, unavailable, remote, or service-backed.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::runtime_context::RuntimeContext;

/// MCP protocol version advertised by a client/server pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpProtocolVersion {
    pub version: String,
}

impl McpProtocolVersion {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }
}

/// HTTP metadata for MCP transports. Query params are separated from headers
/// so snapshots can redact or validate them independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct McpHttpOptions {
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query_params: BTreeMap<String, String>,
}

/// Provider-neutral MCP transport descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpClientTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        cwd: Option<String>,
    },
    Http {
        url: String,
        options: McpHttpOptions,
    },
    Sse {
        url: String,
        options: McpHttpOptions,
    },
    StreamableHttp {
        url: String,
        options: McpHttpOptions,
    },
}

/// Include/exclude filter applied to model-visible MCP tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct McpToolFilter {
    #[serde(default)]
    pub include: BTreeSet<String>,
    #[serde(default)]
    pub exclude: BTreeSet<String>,
}

impl McpToolFilter {
    pub fn allows(&self, tool_name: &str) -> bool {
        (self.include.is_empty() || self.include.contains(tool_name))
            && !self.exclude.contains(tool_name)
    }
}

/// Elicitation support advertised by an MCP client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpElicitationConfig {
    pub enabled: bool,
    pub prompt_channel: Option<String>,
}

impl Default for McpElicitationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prompt_channel: None,
        }
    }
}

/// Stable client descriptor stored in registries and snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpClientDescriptor {
    pub client_id: String,
    pub transport: McpClientTransport,
    pub protocol: McpProtocolVersion,
    pub tool_filter: McpToolFilter,
    pub elicitation: McpElicitationConfig,
}

impl McpClientDescriptor {
    pub fn new(
        client_id: impl Into<String>,
        transport: McpClientTransport,
        protocol: McpProtocolVersion,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            transport,
            protocol,
            tool_filter: McpToolFilter::default(),
            elicitation: McpElicitationConfig::default(),
        }
    }
}

/// Registry command carrying trace/scope for MCP registry operations.
#[derive(Debug, Clone)]
pub struct McpRegistryCommand {
    pub runtime: RuntimeContext,
}

impl McpRegistryCommand {
    pub fn new(runtime: RuntimeContext) -> Self {
        Self { runtime }
    }
}

/// Structured errors for MCP registry operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum McpRegistryError {
    #[error("mcp registry unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("mcp client already exists: {client_id}")]
    AlreadyExists { client_id: String },
    #[error("mcp client not found: {client_id}")]
    NotFound { client_id: String },
}

/// Provider-neutral MCP registry contract.
#[async_trait]
pub trait McpClientRegistry: Send + Sync {
    async fn register(
        &self,
        command: McpRegistryCommand,
        descriptor: McpClientDescriptor,
    ) -> Result<(), McpRegistryError>;
    async fn list(
        &self,
        command: McpRegistryCommand,
    ) -> Result<Vec<McpClientDescriptor>, McpRegistryError>;
    async fn remove(
        &self,
        command: McpRegistryCommand,
        client_id: &str,
    ) -> Result<(), McpRegistryError>;
}

/// In-memory mock registry for deterministic tests and local composition.
pub struct InMemoryMcpClientRegistry {
    descriptors: tokio::sync::RwLock<BTreeMap<String, McpClientDescriptor>>,
}

impl InMemoryMcpClientRegistry {
    pub fn new() -> Self {
        info!("in-memory mcp client registry initialized");
        Self {
            descriptors: tokio::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl Default for InMemoryMcpClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpClientRegistry for InMemoryMcpClientRegistry {
    async fn register(
        &self,
        command: McpRegistryCommand,
        descriptor: McpClientDescriptor,
    ) -> Result<(), McpRegistryError> {
        debug!(
            trace_id = %command.runtime.trace_id,
            client_id = %descriptor.client_id,
            "registering mcp client descriptor"
        );
        let mut descriptors = self.descriptors.write().await;
        if descriptors.contains_key(&descriptor.client_id) {
            return Err(McpRegistryError::AlreadyExists {
                client_id: descriptor.client_id,
            });
        }
        descriptors.insert(descriptor.client_id.clone(), descriptor);
        Ok(())
    }

    async fn list(
        &self,
        command: McpRegistryCommand,
    ) -> Result<Vec<McpClientDescriptor>, McpRegistryError> {
        debug!(trace_id = %command.runtime.trace_id, "listing mcp client descriptors");
        Ok(self.descriptors.read().await.values().cloned().collect())
    }

    async fn remove(
        &self,
        command: McpRegistryCommand,
        client_id: &str,
    ) -> Result<(), McpRegistryError> {
        debug!(trace_id = %command.runtime.trace_id, client_id, "removing mcp client descriptor");
        let removed = self.descriptors.write().await.remove(client_id);
        if removed.is_none() {
            return Err(McpRegistryError::NotFound {
                client_id: client_id.to_string(),
            });
        }
        Ok(())
    }
}

/// Null Object registry for absent MCP providers.
pub struct UnavailableMcpClientRegistry {
    reason: String,
}

impl UnavailableMcpClientRegistry {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    fn unavailable(&self) -> McpRegistryError {
        McpRegistryError::Unavailable {
            reason: self.reason.clone(),
        }
    }
}

#[async_trait]
impl McpClientRegistry for UnavailableMcpClientRegistry {
    async fn register(
        &self,
        command: McpRegistryCommand,
        _descriptor: McpClientDescriptor,
    ) -> Result<(), McpRegistryError> {
        warn!(trace_id = %command.runtime.trace_id, reason = %self.reason, "mcp registry unavailable");
        Err(self.unavailable())
    }

    async fn list(
        &self,
        command: McpRegistryCommand,
    ) -> Result<Vec<McpClientDescriptor>, McpRegistryError> {
        warn!(trace_id = %command.runtime.trace_id, reason = %self.reason, "mcp registry unavailable");
        Err(self.unavailable())
    }

    async fn remove(
        &self,
        command: McpRegistryCommand,
        _client_id: &str,
    ) -> Result<(), McpRegistryError> {
        warn!(trace_id = %command.runtime.trace_id, reason = %self.reason, "mcp registry unavailable");
        Err(self.unavailable())
    }
}

#[cfg(test)]
#[path = "mcp_contract_tests.rs"]
mod mcp_contract_tests;
