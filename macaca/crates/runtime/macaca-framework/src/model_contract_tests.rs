// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

#[tokio::test]
async fn model_registry_selects_requested_capable_card() {
    let registry = InMemoryModelRegistry::new();
    let card = ModelCard::new("model-a", "route-a")
        .with_capability(ModelCapability::Text)
        .with_capability(ModelCapability::ToolUse);

    registry.register(card.clone()).await.unwrap();
    let selected = registry
        .select(
            ModelSelectionCommand::new(RuntimeContext::new("trace-model-select"))
                .requested_model_id("model-a")
                .require(ModelCapability::ToolUse),
        )
        .await
        .unwrap();

    assert_eq!(selected, card);
}

#[tokio::test]
async fn model_registry_rejects_missing_capability() {
    let registry = InMemoryModelRegistry::new();
    registry
        .register(ModelCard::new("model-a", "route-a").with_capability(ModelCapability::Text))
        .await
        .unwrap();

    let error = registry
        .select(
            ModelSelectionCommand::new(RuntimeContext::new("trace-model-unsupported"))
                .requested_model_id("model-a")
                .require(ModelCapability::StructuredOutput),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, ModelRegistryError::Unsupported { .. }));
}

#[tokio::test]
async fn credential_contract_stores_secret_references_only() {
    let credential = ModelCredentialRef::new("secret://tenant/model");
    let card = ModelCard::new("model-a", "route-a").with_capability(ModelCapability::Text);
    let mut card = card;
    card.credential_ref = Some(credential);

    let serialized = serde_json::to_string(&card).unwrap();

    assert!(serialized.contains("secret://tenant/model"));
    assert!(!serialized.contains("api_key"));
}

#[tokio::test]
async fn unavailable_model_registry_fails_explicitly() {
    let registry = UnavailableModelRegistry::new("llm service not installed");
    let error = registry
        .select(ModelSelectionCommand::new(RuntimeContext::new(
            "trace-model-unavailable",
        )))
        .await
        .unwrap_err();

    assert!(matches!(error, ModelRegistryError::Unavailable { .. }));
}
