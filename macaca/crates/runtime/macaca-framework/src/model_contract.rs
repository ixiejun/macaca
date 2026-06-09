// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Provider-neutral model registry and credential contracts.
//!
//! The framework may describe, select, and validate model routes, but concrete
//! LLM provider construction belongs to service/runtime-host composition roots.
//! These contracts use the Specification and Registry patterns so future
//! AgentScope or Macaca provider upgrades can add model cards without changing
//! agent execution code or leaking raw credentials into framework state.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::runtime_context::RuntimeContext;

/// Capability flags that a model route may advertise.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Text,
    ToolUse,
    Streaming,
    StructuredOutput,
    MultimodalInput,
    MultimodalOutput,
    ReasoningTrace,
}

/// Credential reference for a model route.
///
/// This type intentionally stores only secret references and policy labels.
/// Raw API keys, tokens, provider payloads, or tenant credentials must remain
/// in the service credential store and never enter framework snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCredentialRef {
    pub secret_ref: String,
    #[serde(default)]
    pub required_scopes: BTreeSet<String>,
}

impl ModelCredentialRef {
    pub fn new(secret_ref: impl Into<String>) -> Self {
        Self {
            secret_ref: secret_ref.into(),
            required_scopes: BTreeSet::new(),
        }
    }
}

/// Provider-neutral model card visible to framework providers and shells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCard {
    pub model_id: String,
    pub route_ref: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub context_window: Option<u32>,
    #[serde(default)]
    pub output_limit: Option<u32>,
    #[serde(default)]
    pub capabilities: BTreeSet<ModelCapability>,
    #[serde(default)]
    pub credential_ref: Option<ModelCredentialRef>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl ModelCard {
    pub fn new(model_id: impl Into<String>, route_ref: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            route_ref: route_ref.into(),
            display_name: None,
            context_window: None,
            output_limit: None,
            capabilities: BTreeSet::new(),
            credential_ref: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_capability(mut self, capability: ModelCapability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    fn supports(&self, required: &BTreeSet<ModelCapability>) -> bool {
        required.is_subset(&self.capabilities)
    }
}

/// Model route selection command. The runtime context supplies trace and scope.
#[derive(Debug, Clone)]
pub struct ModelSelectionCommand {
    pub runtime: RuntimeContext,
    pub requested_model_id: Option<String>,
    pub required_capabilities: BTreeSet<ModelCapability>,
}

impl ModelSelectionCommand {
    pub fn new(runtime: RuntimeContext) -> Self {
        Self {
            runtime,
            requested_model_id: None,
            required_capabilities: BTreeSet::new(),
        }
    }

    pub fn requested_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.requested_model_id = Some(model_id.into());
        self
    }

    pub fn require(mut self, capability: ModelCapability) -> Self {
        self.required_capabilities.insert(capability);
        self
    }
}

/// Structured registry errors returned instead of provider-specific failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelRegistryError {
    #[error("model registry unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("model card already exists: {model_id}")]
    AlreadyExists { model_id: String },
    #[error("model card not found: {model_id}")]
    NotFound { model_id: String },
    #[error("model route unsupported: {reason}")]
    Unsupported { reason: String },
}

/// Provider-neutral model registry. Runtime-host may back this with LLM
/// service catalogs, plugin registries, remote discovery, mocks, or unavailable
/// providers without changing framework callers.
#[async_trait]
pub trait ModelRegistry: Send + Sync {
    async fn register(&self, card: ModelCard) -> Result<(), ModelRegistryError>;
    async fn remove(&self, model_id: &str) -> Result<(), ModelRegistryError>;
    async fn get(&self, model_id: &str) -> Result<Option<ModelCard>, ModelRegistryError>;
    async fn list(&self) -> Result<Vec<ModelCard>, ModelRegistryError>;
    async fn select(&self, command: ModelSelectionCommand)
        -> Result<ModelCard, ModelRegistryError>;
}

/// Deterministic in-memory registry for tests and local composition.
pub struct InMemoryModelRegistry {
    cards: tokio::sync::RwLock<BTreeMap<String, ModelCard>>,
}

impl InMemoryModelRegistry {
    pub fn new() -> Self {
        info!("in-memory model registry initialized");
        Self {
            cards: tokio::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl Default for InMemoryModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelRegistry for InMemoryModelRegistry {
    async fn register(&self, card: ModelCard) -> Result<(), ModelRegistryError> {
        debug!(model_id = %card.model_id, route_ref = %card.route_ref, "registering model card");
        let mut cards = self.cards.write().await;
        if cards.contains_key(&card.model_id) {
            return Err(ModelRegistryError::AlreadyExists {
                model_id: card.model_id,
            });
        }
        cards.insert(card.model_id.clone(), card);
        Ok(())
    }

    async fn remove(&self, model_id: &str) -> Result<(), ModelRegistryError> {
        debug!(model_id, "removing model card");
        self.cards.write().await.remove(model_id);
        Ok(())
    }

    async fn get(&self, model_id: &str) -> Result<Option<ModelCard>, ModelRegistryError> {
        Ok(self.cards.read().await.get(model_id).cloned())
    }

    async fn list(&self) -> Result<Vec<ModelCard>, ModelRegistryError> {
        Ok(self.cards.read().await.values().cloned().collect())
    }

    async fn select(
        &self,
        command: ModelSelectionCommand,
    ) -> Result<ModelCard, ModelRegistryError> {
        debug!(
            trace_id = %command.runtime.trace_id,
            requested_model_id = ?command.requested_model_id,
            "selecting model card"
        );
        let cards = self.cards.read().await;
        if let Some(model_id) = command.requested_model_id {
            let card = cards
                .get(&model_id)
                .cloned()
                .ok_or(ModelRegistryError::NotFound { model_id })?;
            if card.supports(&command.required_capabilities) {
                return Ok(card);
            }
            return Err(ModelRegistryError::Unsupported {
                reason: "requested model does not satisfy required capabilities".to_string(),
            });
        }
        cards
            .values()
            .find(|card| card.supports(&command.required_capabilities))
            .cloned()
            .ok_or_else(|| ModelRegistryError::Unsupported {
                reason: "no registered model satisfies required capabilities".to_string(),
            })
    }
}

/// Null Object registry for absent model-service providers.
pub struct UnavailableModelRegistry {
    reason: String,
}

impl UnavailableModelRegistry {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ModelRegistry for UnavailableModelRegistry {
    async fn register(&self, _card: ModelCard) -> Result<(), ModelRegistryError> {
        Err(self.unavailable())
    }

    async fn remove(&self, _model_id: &str) -> Result<(), ModelRegistryError> {
        Err(self.unavailable())
    }

    async fn get(&self, _model_id: &str) -> Result<Option<ModelCard>, ModelRegistryError> {
        Err(self.unavailable())
    }

    async fn list(&self) -> Result<Vec<ModelCard>, ModelRegistryError> {
        Err(self.unavailable())
    }

    async fn select(
        &self,
        _command: ModelSelectionCommand,
    ) -> Result<ModelCard, ModelRegistryError> {
        warn!(reason = %self.reason, "model registry unavailable");
        Err(self.unavailable())
    }
}

impl UnavailableModelRegistry {
    fn unavailable(&self) -> ModelRegistryError {
        ModelRegistryError::Unavailable {
            reason: self.reason.clone(),
        }
    }
}

#[cfg(test)]
#[path = "model_contract_tests.rs"]
mod model_contract_tests;
