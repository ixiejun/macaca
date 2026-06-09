// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

#[test]
fn default_descriptor_tracks_contract_and_license() {
    let descriptor = FrameworkDescriptor::default_contract();

    assert_eq!(descriptor.provider_id, DEFAULT_FRAMEWORK_PROVIDER_ID);
    assert_eq!(
        descriptor.contract_version,
        MACACA_FRAMEWORK_CONTRACT_VERSION
    );
    assert_eq!(descriptor.upstream_name.as_deref(), Some("AgentScope Java"));
    assert!(descriptor.apache_2_notice_required);
    assert!(!descriptor.capabilities.entries.is_empty());
}

#[test]
fn contract_matrix_reports_completed_rows() {
    let matrix = FrameworkCapabilityMatrix::contract_only();

    assert_eq!(matrix.unavailable_count(), 0);
    assert!(matrix.entries.iter().any(|entry| {
        entry.capability == FrameworkCapability::LicenseCompliance
            && entry.status == FrameworkCapabilityStatus::Equivalent
    }));
    assert!(matrix.entries.iter().any(|entry| {
        entry.capability == FrameworkCapability::HarnessFilesystemSandbox
            && entry.status == FrameworkCapabilityStatus::DelegatedVerified
    }));
}

#[test]
fn capability_matrix_has_fine_grained_evidence() {
    let matrix = FrameworkCapabilityMatrix::contract_only();

    assert!(
        matrix.rows_without_positive_evidence().is_empty(),
        "every claimed capability must carry evidence and test refs"
    );
    assert!(matrix.entries.iter().any(|entry| {
        entry.capability == FrameworkCapability::ModelTransportContract
            && entry.status == FrameworkCapabilityStatus::ContractOnly
            && entry.delegation_target.is_none()
    }));
    assert!(matrix.entries.iter().any(|entry| {
        entry.capability == FrameworkCapability::Mcp
            && entry.delegation_target.as_deref() == Some("mcp-service/runtime-host adapter")
    }));
}

#[tokio::test]
async fn unavailable_provider_returns_structured_unavailable() {
    let provider = UnavailableAgentRuntimeProvider::new("not registered");
    let health = provider.health();

    assert_eq!(health.state, FrameworkHealthState::Unavailable);
    assert_eq!(provider.snapshot().in_flight_call_count, 0);

    let error = provider
        .call(AgentRuntimeCallCommand {
            target_agent: "runtime-agent".to_string(),
            input: Vec::new(),
            trace_id: "trace-contract".to_string(),
        })
        .await
        .expect_err("unavailable provider must reject calls structurally");

    assert!(matches!(error, FrameworkProviderError::Unavailable { .. }));
}

#[tokio::test]
async fn unavailable_stream_returns_structured_unavailable() {
    let provider = UnavailableAgentRuntimeProvider::new("not registered");

    let error = provider
        .stream_events(AgentRuntimeStreamCommand {
            target_agent: "runtime-agent".to_string(),
            input: Vec::new(),
            trace_id: "trace-stream-contract".to_string(),
        })
        .await
        .expect_err("unavailable provider must reject stream calls structurally");

    assert!(matches!(error, FrameworkProviderError::Unavailable { .. }));
}
