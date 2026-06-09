// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It intentionally exposes
// provider-neutral contracts instead of AgentScope-specific structs so callers
// can replace the framework provider without changing Macaca consumer code.

//! Provider-neutral Agent Framework contract.
//!
//! This module defines the stable Macaca ABI that the default AgentScope
//! 2.0-equivalent provider and future mock, remote, plugin, or unavailable
//! providers must implement.
//!
//! # Design patterns
//!
//! - **Facade**: consumers see `AgentRuntimeProvider`, not a concrete framework.
//! - **Command**: calls use typed command/result values.
//! - **Adapter/Bridge**: an AgentScope 2.0 implementation can sit behind this trait.
//! - **State/Memento**: snapshots expose bounded provider state for diagnostics.
//! - **Null Object**: `UnavailableAgentRuntimeProvider` is explicit absent behavior.

#[path = "provider_capability_matrix.rs"]
mod provider_capability_matrix;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::event_contract::AgentEvent;
use crate::message::Msg;

/// Stable id used by the built-in default framework provider family.
///
/// This is a capability/provider id, not a business or application name. It is
/// safe at the framework boundary because callers use it for diagnostics and
/// provider selection, never for application-specific routing logic.
pub const DEFAULT_FRAMEWORK_PROVIDER_ID: &str = "framework.default";

/// Version of the provider-neutral Macaca framework contract.
///
/// Keep this separate from the upstream AgentScope version so a future
/// AgentScope upgrade can keep the same Macaca ABI when consumer behavior does
/// not need to change.
pub const MACACA_FRAMEWORK_CONTRACT_VERSION: &str = "2.0.0-macaca.0";

/// Provider implementation class for diagnostics and replacement policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkProviderKind {
    BuiltIn,
    Plugin,
    Remote,
    Mock,
    Unavailable,
}

/// Framework capability areas tracked by the upgrade matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkCapability {
    Descriptor,
    Health,
    Snapshot,
    MessageBlocks,
    AgentEvents,
    AgentRuntimeContracts,
    UserInputHitl,
    StreamOptionsStructuredOutput,
    RuntimeContext,
    AgentStateSession,
    MiddlewareStages,
    BuiltInMiddleware,
    ReActLoop,
    ModelRegistry,
    ModelFormatterContract,
    ModelTransportContract,
    ModelExceptionTaxonomy,
    Toolkit,
    ToolContextInjection,
    ToolSuspendState,
    PermissionHitl,
    Mcp,
    McpContentConversion,
    HarnessWorkspace,
    HarnessMemoryContext,
    HarnessFilesystemSandbox,
    HarnessSessionTree,
    HarnessSkills,
    HarnessSubagents,
    HarnessPlanMode,
    ProtocolAdapters,
    AgentProtocolProjection,
    LicenseCompliance,
}

/// Evidence-backed status of a capability in a concrete provider.
///
/// The old broad `Available` status hid partial AgentScope parity. These states
/// force providers to distinguish framework-owned equivalents from delegated
/// service contracts and explicitly missing or policy-disabled behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkCapabilityStatus {
    Equivalent,
    ContractOnly,
    DelegatedVerified,
    DelegatedUnverified,
    Missing,
    UnsupportedByPolicy,
}

/// One capability-matrix row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkCapabilityEntry {
    pub capability: FrameworkCapability,
    pub status: FrameworkCapabilityStatus,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    pub delegation_target: Option<String>,
    pub test_coverage_refs: Vec<String>,
    pub known_limitations: Vec<String>,
}

impl FrameworkCapabilityEntry {
    /// Build a matrix row with bounded human-readable notes.
    pub fn new(
        capability: FrameworkCapability,
        status: FrameworkCapabilityStatus,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            capability,
            status,
            notes: notes.into(),
            evidence_refs: Vec::new(),
            delegation_target: None,
            test_coverage_refs: Vec::new(),
            known_limitations: Vec::new(),
        }
    }

    /// Attach a stable evidence reference for audit and future upgrade review.
    pub fn with_evidence(mut self, evidence_ref: impl Into<String>) -> Self {
        self.evidence_refs.push(evidence_ref.into());
        self
    }

    /// Attach the service/runtime-host boundary that owns concrete side effects.
    pub fn with_delegation_target(mut self, target: impl Into<String>) -> Self {
        self.delegation_target = Some(target.into());
        self
    }

    /// Attach a test or gate reference that proves the capability state.
    pub fn with_test_ref(mut self, test_ref: impl Into<String>) -> Self {
        self.test_coverage_refs.push(test_ref.into());
        self
    }

    /// Attach a sanitized known limitation for snapshots and maintainer review.
    pub fn with_limitation(mut self, limitation: impl Into<String>) -> Self {
        self.known_limitations.push(limitation.into());
        self
    }

    /// Return true when this row can be claimed as framework parity.
    pub fn has_positive_evidence(&self) -> bool {
        matches!(
            self.status,
            FrameworkCapabilityStatus::Equivalent
                | FrameworkCapabilityStatus::ContractOnly
                | FrameworkCapabilityStatus::DelegatedVerified
        ) && !self.evidence_refs.is_empty()
            && !self.test_coverage_refs.is_empty()
    }
}

/// Provider capability matrix used for health, audit, and upgrade tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkCapabilityMatrix {
    pub entries: Vec<FrameworkCapabilityEntry>,
}

/// Coarse provider health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkHealthState {
    Healthy,
    Degraded,
    Unavailable,
}

/// Bounded health result for diagnostics and service snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkHealth {
    pub provider_id: String,
    pub state: FrameworkHealthState,
    pub reason: String,
    pub capability_count: usize,
    pub unavailable_capability_count: usize,
}

/// Descriptor returned by every framework provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkDescriptor {
    pub provider_id: String,
    pub display_name: String,
    pub contract_version: String,
    pub upstream_name: Option<String>,
    pub upstream_version: Option<String>,
    pub kind: FrameworkProviderKind,
    pub apache_2_notice_required: bool,
    pub capabilities: FrameworkCapabilityMatrix,
}

impl FrameworkDescriptor {
    /// Descriptor for the first AgentScope 2.0-equivalent contract slice.
    pub fn default_contract() -> Self {
        Self {
            provider_id: DEFAULT_FRAMEWORK_PROVIDER_ID.to_string(),
            display_name: "AgentScope 2.0-equivalent Macaca framework provider".to_string(),
            contract_version: MACACA_FRAMEWORK_CONTRACT_VERSION.to_string(),
            upstream_name: Some("AgentScope Java".to_string()),
            upstream_version: Some("2.0.0-RC1".to_string()),
            kind: FrameworkProviderKind::BuiltIn,
            apache_2_notice_required: true,
            capabilities: FrameworkCapabilityMatrix::contract_only(),
        }
    }

    /// Descriptor for explicit unavailable provider behavior.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let mut descriptor = Self::default_contract();
        descriptor.kind = FrameworkProviderKind::Unavailable;
        descriptor.display_name = format!("Unavailable framework provider: {reason}");
        descriptor
    }
}

/// Sanitized provider snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkSnapshot {
    pub descriptor: FrameworkDescriptor,
    pub health: FrameworkHealth,
    pub active_session_count: usize,
    pub in_flight_call_count: usize,
    pub sanitized_notes: Vec<String>,
}

/// Structured framework-provider errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum FrameworkProviderError {
    #[error("framework provider unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("framework command unsupported: {command}")]
    Unsupported { command: String },
    #[error("framework command denied: {reason}")]
    Denied { reason: String },
    #[error("framework execution failed: {reason}")]
    Failed { reason: String },
}

/// Typed command for final-message call projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeCallCommand {
    pub target_agent: String,
    pub input: Vec<Msg>,
    pub trace_id: String,
}

/// Typed result for a framework call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeCallResult {
    pub final_message: Option<Msg>,
    pub generated_event_count: usize,
    pub provider_id: String,
}

/// Typed command for canonical event-stream execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeStreamCommand {
    pub target_agent: String,
    pub input: Vec<Msg>,
    pub trace_id: String,
}

impl From<AgentRuntimeCallCommand> for AgentRuntimeStreamCommand {
    fn from(command: AgentRuntimeCallCommand) -> Self {
        Self {
            target_agent: command.target_agent,
            input: command.input,
            trace_id: command.trace_id,
        }
    }
}

/// Event-stream result plus projected final message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeStreamResult {
    pub events: Vec<AgentEvent>,
    pub final_message: Option<Msg>,
    pub provider_id: String,
}

impl AgentRuntimeStreamResult {
    /// Project canonical events into the final-message call result.
    pub fn into_call_result(self) -> AgentRuntimeCallResult {
        AgentRuntimeCallResult {
            generated_event_count: self.events.len(),
            final_message: self.final_message,
            provider_id: self.provider_id,
        }
    }
}

/// Provider-neutral framework execution facade.
#[async_trait]
pub trait AgentRuntimeProvider: Send + Sync {
    /// Return immutable provider metadata.
    fn descriptor(&self) -> FrameworkDescriptor;

    /// Return a bounded health result safe for logs and shell diagnostics.
    fn health(&self) -> FrameworkHealth;

    /// Return a sanitized state snapshot.
    fn snapshot(&self) -> FrameworkSnapshot;

    /// Execute one final-message call through the provider.
    async fn call(
        &self,
        command: AgentRuntimeCallCommand,
    ) -> Result<AgentRuntimeCallResult, FrameworkProviderError>;

    /// Execute one call and return canonical typed events.
    async fn stream_events(
        &self,
        command: AgentRuntimeStreamCommand,
    ) -> Result<AgentRuntimeStreamResult, FrameworkProviderError>;
}

/// Null-object provider for absent Agent Framework capability.
pub struct UnavailableAgentRuntimeProvider {
    descriptor: FrameworkDescriptor,
    reason: String,
}

impl UnavailableAgentRuntimeProvider {
    /// Build an explicit unavailable provider. This keeps missing framework
    /// capability observable and auditable instead of crashing at call sites.
    pub fn new(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        info!(
            provider_id = DEFAULT_FRAMEWORK_PROVIDER_ID,
            reason = %reason,
            "unavailable agent runtime provider initialized"
        );
        Self {
            descriptor: FrameworkDescriptor::unavailable(reason.clone()),
            reason,
        }
    }
}

#[async_trait]
impl AgentRuntimeProvider for UnavailableAgentRuntimeProvider {
    fn descriptor(&self) -> FrameworkDescriptor {
        self.descriptor.clone()
    }

    fn health(&self) -> FrameworkHealth {
        let capability_count = self.descriptor.capabilities.entries.len();
        FrameworkHealth {
            provider_id: self.descriptor.provider_id.clone(),
            state: FrameworkHealthState::Unavailable,
            reason: self.reason.clone(),
            capability_count,
            unavailable_capability_count: capability_count,
        }
    }

    fn snapshot(&self) -> FrameworkSnapshot {
        FrameworkSnapshot {
            descriptor: self.descriptor(),
            health: self.health(),
            active_session_count: 0,
            in_flight_call_count: 0,
            sanitized_notes: vec![self.reason.clone()],
        }
    }

    async fn call(
        &self,
        command: AgentRuntimeCallCommand,
    ) -> Result<AgentRuntimeCallResult, FrameworkProviderError> {
        warn!(
            provider_id = %self.descriptor.provider_id,
            target_agent = %command.target_agent,
            trace_id = %command.trace_id,
            reason = %self.reason,
            "framework call rejected because provider is unavailable"
        );
        Err(FrameworkProviderError::Unavailable {
            reason: self.reason.clone(),
        })
    }

    async fn stream_events(
        &self,
        command: AgentRuntimeStreamCommand,
    ) -> Result<AgentRuntimeStreamResult, FrameworkProviderError> {
        warn!(
            provider_id = %self.descriptor.provider_id,
            target_agent = %command.target_agent,
            trace_id = %command.trace_id,
            reason = %self.reason,
            "framework stream rejected because provider is unavailable"
        );
        Err(FrameworkProviderError::Unavailable {
            reason: self.reason.clone(),
        })
    }
}

#[cfg(test)]
#[path = "provider_contract_tests.rs"]
mod provider_contract_tests;
