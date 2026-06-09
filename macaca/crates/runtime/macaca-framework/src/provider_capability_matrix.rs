// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Evidence-backed AgentScope 2.0 capability matrix construction.
//!
//! This module keeps the large static parity table out of the provider facade
//! file. The table is deliberately data-only: it records framework contracts,
//! delegated service targets, and test evidence without constructing concrete
//! providers or branching on application-specific behavior.

use super::{
    FrameworkCapability, FrameworkCapabilityEntry, FrameworkCapabilityMatrix,
    FrameworkCapabilityStatus,
};

impl FrameworkCapabilityMatrix {
    /// Matrix for the completed AgentScope 2.0-equivalent provider contract.
    pub fn contract_only() -> Self {
        Self {
            entries: vec![
                equivalent(
                    FrameworkCapability::Descriptor,
                    "provider descriptor contract is available",
                    "src/provider_contract.rs:FrameworkDescriptor",
                    "provider_contract_tests::default_descriptor_tracks_contract_and_license",
                ),
                equivalent(
                    FrameworkCapability::Health,
                    "provider health contract is available",
                    "src/provider_contract.rs:FrameworkHealth",
                    "provider_contract_tests::default_descriptor_tracks_contract_and_license",
                ),
                equivalent(
                    FrameworkCapability::Snapshot,
                    "provider snapshot contract is available",
                    "src/provider_contract.rs:FrameworkSnapshot",
                    "provider_contract_tests::capability_matrix_has_fine_grained_evidence",
                ),
                equivalent(
                    FrameworkCapability::MessageBlocks,
                    "AgentScope 2.0 role validation helpers plus DataBlock and HintBlock are available",
                    "src/message.rs:ContentBlock",
                    "message_tests",
                ),
                equivalent(
                    FrameworkCapability::AgentEvents,
                    "typed AgentEvent contract, replay accumulator, and canonical stream projection are available",
                    "src/event_contract.rs:AgentEvent",
                    "event_contract_tests",
                ),
                contract_only(
                    FrameworkCapability::AgentRuntimeContracts,
                    "callable, streamable, and observable agent contracts make reply a stream projection",
                    "src/agent_contracts.rs",
                    "agent_contracts_tests",
                ),
                contract_only(
                    FrameworkCapability::UserInputHitl,
                    "UserAgent/StreamUserInput/HITL resume contracts are represented as typed pending input state",
                    "src/agent_contracts.rs:PendingInputState",
                    "agent_contracts_tests",
                ),
                contract_only(
                    FrameworkCapability::StreamOptionsStructuredOutput,
                    "StreamOptions and structured output request contracts are provider-neutral",
                    "src/agent_contracts.rs:StreamOptions",
                    "agent_contracts_tests",
                ),
                equivalent(
                    FrameworkCapability::RuntimeContext,
                    "RuntimeContext, SessionKey, AgentState, and session-store contracts are available",
                    "src/runtime_context.rs:RuntimeContext",
                    "runtime_context_tests",
                ),
                equivalent(
                    FrameworkCapability::AgentStateSession,
                    "provider-neutral durable AgentState and AgentSessionStore contracts are available",
                    "src/runtime_context.rs:AgentState",
                    "runtime_context_tests",
                ),
                equivalent(
                    FrameworkCapability::MiddlewareStages,
                    "five-stage middleware contract is available",
                    "src/middleware.rs:AgentMiddlewareChain",
                    "middleware::tests",
                ),
                contract_only(
                    FrameworkCapability::BuiltInMiddleware,
                    "trace, reminder, system-prompt, and RAG middleware contracts are provider-neutral and side-effect free",
                    "src/middleware.rs",
                    "middleware::tests",
                )
                .with_limitation("concrete retrieval and task mutation remain delegated services"),
                equivalent(
                    FrameworkCapability::ReActLoop,
                    "ReActAgent event-stream loop, retry/fallback, HITL, external resume, and shutdown paths are available",
                    "src/react_agent.rs:ReActAgent",
                    "react_agent_stream_tests",
                ),
                delegated(
                    FrameworkCapability::ModelRegistry,
                    "model routing remains delegated to LLM service/runtime-host adapters",
                    "src/model_contract.rs:ModelRegistry",
                    "model_contract_tests",
                    "llm-service/runtime-host model registry adapter",
                ),
                contract_only(
                    FrameworkCapability::ModelFormatterContract,
                    "formatter/parser family refs describe provider-specific formatting without provider-name routing",
                    "src/model_transport_contract.rs:FormatterFamilyRef",
                    "model_transport_contract_tests",
                ),
                contract_only(
                    FrameworkCapability::ModelTransportContract,
                    "HTTP/WebSocket/service-bus transport DTOs are defined without concrete clients",
                    "src/model_transport_contract.rs:ModelTransportCommand",
                    "model_transport_contract_tests",
                ),
                contract_only(
                    FrameworkCapability::ModelExceptionTaxonomy,
                    "model errors are normalized into auth, bad request, rate limit, not found, permission, timeout, unavailable, internal, provider failure, and unsupported states",
                    "src/model_transport_contract.rs:ModelException",
                    "model_transport_contract_tests",
                ),
                equivalent(
                    FrameworkCapability::Toolkit,
                    "AgentScope 2.0 tool contracts and guarded toolkit bridge are available",
                    "src/tool_contract.rs:ToolSpec",
                    "tool_contract_tests",
                ),
                contract_only(
                    FrameworkCapability::ToolContextInjection,
                    "ToolRuntimeContext injects trace/runtime metadata before delegated side effects",
                    "src/tool_contract.rs:ToolRuntimeContext",
                    "tool_contract_tests",
                ),
                contract_only(
                    FrameworkCapability::ToolSuspendState,
                    "tool suspend/external execution state machine uses correlation and idempotency contracts",
                    "src/tool_contract.rs:ToolExecutionResult",
                    "tool_contract_tests",
                ),
                equivalent(
                    FrameworkCapability::PermissionHitl,
                    "permission ask/deny/external suspend and resume state handling are available",
                    "src/permission_contract.rs",
                    "permission_contract_tests",
                ),
                delegated(
                    FrameworkCapability::Mcp,
                    "AgentScope 2.0 MCP descriptor registry and runtime-host synchronization are available",
                    "src/mcp_contract.rs:McpClientRegistry",
                    "mcp_contract_tests",
                    "mcp-service/runtime-host adapter",
                ),
                contract_only(
                    FrameworkCapability::McpContentConversion,
                    "MCP content conversion maps service-returned content into ContentBlock and ToolExecutionResult",
                    "src/mcp_content_contract.rs",
                    "mcp_content_contract_tests",
                ),
                equivalent(
                    FrameworkCapability::HarnessWorkspace,
                    "Harness workspace wrapper and system-prompt middleware are available",
                    "src/harness.rs:WorkspaceManager",
                    "harness_tests",
                ),
                delegated(
                    FrameworkCapability::HarnessMemoryContext,
                    "Harness compaction, eviction, overflow, and memory flush ports are available",
                    "src/harness_context.rs:HarnessMemoryPort",
                    "harness_tests",
                    "memory/context services",
                ),
                delegated(
                    FrameworkCapability::HarnessFilesystemSandbox,
                    "Harness filesystem and sandbox execution are delegated to service ports with unavailable adapters",
                    "src/harness_filesystem.rs",
                    "harness_contracts_tests",
                    "filesystem/sandbox services or runtime-host providers",
                ),
                contract_only(
                    FrameworkCapability::HarnessSessionTree,
                    "session tree, freshness, restore, and checkpoint mementos are represented as neutral DTOs",
                    "src/harness_contracts.rs:HarnessSessionNode",
                    "harness_contracts_tests",
                ),
                delegated(
                    FrameworkCapability::HarnessSkills,
                    "skill repositories remain delegated to skill/plugin services",
                    "src/harness_skill.rs:HarnessSkillRepositoryPort",
                    "harness_contracts_tests",
                    "skill service/plugin repository",
                ),
                delegated(
                    FrameworkCapability::HarnessSubagents,
                    "Harness subagent declarations, delegation envelopes, and task repository ports are available",
                    "src/harness_subagent.rs:HarnessSubagentPort",
                    "harness_contracts_tests",
                    "task/execution-control services",
                ),
                equivalent(
                    FrameworkCapability::HarnessPlanMode,
                    "Harness plan mode uses agent-local state behind task service boundaries",
                    "src/harness_plan.rs",
                    "plan_tests",
                ),
                equivalent(
                    FrameworkCapability::ProtocolAdapters,
                    "A2A, AG-UI, Chat Completions, and optional extension adapters consume AgentEvent",
                    "src/protocol_adapters.rs",
                    "protocol_adapters_tests",
                ),
                contract_only(
                    FrameworkCapability::AgentProtocolProjection,
                    "Agent Protocol projection observes AgentEvent and preserves trace linkage",
                    "src/protocol_adapters.rs",
                    "protocol_adapters_tests",
                )
                .with_limitation("wire transport is owned by gateway/service adapters"),
                equivalent(
                    FrameworkCapability::LicenseCompliance,
                    "new adapted files include Apache-2.0 source notice headers",
                    "tests/agentscope2_license_headers.rs",
                    "agentscope2_license_headers",
                ),
            ],
        }
    }

    /// Count rows that cannot be used as a verified provider capability.
    pub fn unavailable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.status,
                    FrameworkCapabilityStatus::Missing
                        | FrameworkCapabilityStatus::UnsupportedByPolicy
                        | FrameworkCapabilityStatus::DelegatedUnverified
                )
            })
            .count()
    }

    /// Return rows that cannot be claimed as completed AgentScope parity.
    pub fn rows_without_positive_evidence(&self) -> Vec<&FrameworkCapabilityEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.has_positive_evidence())
            .collect()
    }
}

fn equivalent(
    capability: FrameworkCapability,
    notes: &'static str,
    evidence: &'static str,
    test_ref: &'static str,
) -> FrameworkCapabilityEntry {
    FrameworkCapabilityEntry::new(capability, FrameworkCapabilityStatus::Equivalent, notes)
        .with_evidence(evidence)
        .with_test_ref(test_ref)
}

fn contract_only(
    capability: FrameworkCapability,
    notes: &'static str,
    evidence: &'static str,
    test_ref: &'static str,
) -> FrameworkCapabilityEntry {
    FrameworkCapabilityEntry::new(capability, FrameworkCapabilityStatus::ContractOnly, notes)
        .with_evidence(evidence)
        .with_test_ref(test_ref)
}

fn delegated(
    capability: FrameworkCapability,
    notes: &'static str,
    evidence: &'static str,
    test_ref: &'static str,
    target: &'static str,
) -> FrameworkCapabilityEntry {
    FrameworkCapabilityEntry::new(
        capability,
        FrameworkCapabilityStatus::DelegatedVerified,
        notes,
    )
    .with_evidence(evidence)
    .with_test_ref(test_ref)
    .with_delegation_target(target)
}
