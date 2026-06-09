// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Harness subagent delegation and task repository contracts.
//!
//! Framework Harness describes sync/background delegation and event forwarding,
//! but concrete subagent factories, task persistence, scheduling, recovery, and
//! routing belong to task/execution-control services.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::agent::AgentResult;
use crate::event_contract::AgentEvent;
use crate::message::Msg;
use crate::runtime_context::RuntimeContext;

/// Delegation mode requested by a Harness caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSubagentDelegationMode {
    Sync,
    Background,
}

/// Command for delegating one message to a subagent service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSubagentDelegationCommand {
    pub runtime: RuntimeContext,
    pub subagent_id: String,
    pub mode: HarnessSubagentDelegationMode,
    pub input: Msg,
}

/// Stable event forwarding envelope. The source id is explicit so merged event
/// streams remain replayable after background task resume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessSubagentEvent {
    pub event_source_id: String,
    pub event: AgentEvent,
}

/// Result of a subagent delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSubagentDelegationResult {
    pub task_id: Option<String>,
    pub final_message: Option<Msg>,
    pub events: Vec<HarnessSubagentEvent>,
    pub reason: Option<String>,
}

/// Replaceable subagent delegation port.
#[async_trait]
pub trait HarnessSubagentPort: Send + Sync {
    fn name(&self) -> &str;
    async fn delegate(
        &self,
        command: HarnessSubagentDelegationCommand,
    ) -> AgentResult<HarnessSubagentDelegationResult>;
}

/// Durable task record metadata for background subagent work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSubagentTaskRecord {
    pub task_id: String,
    pub subagent_id: String,
    pub event_source_id: String,
    pub status: String,
}

/// Replaceable task repository boundary.
#[async_trait]
pub trait HarnessSubagentTaskRepositoryPort: Send + Sync {
    fn name(&self) -> &str;
    async fn save(
        &self,
        runtime: RuntimeContext,
        record: HarnessSubagentTaskRecord,
    ) -> AgentResult<()>;
    async fn get(
        &self,
        runtime: RuntimeContext,
        task_id: &str,
    ) -> AgentResult<Option<HarnessSubagentTaskRecord>>;
}

/// Null Object subagent port for absent subagent services.
pub struct UnavailableHarnessSubagentPort {
    reason: String,
}

impl UnavailableHarnessSubagentPort {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HarnessSubagentPort for UnavailableHarnessSubagentPort {
    fn name(&self) -> &str {
        "unavailable_harness_subagent"
    }

    async fn delegate(
        &self,
        command: HarnessSubagentDelegationCommand,
    ) -> AgentResult<HarnessSubagentDelegationResult> {
        warn!(
            trace_id = %command.runtime.trace_id,
            subagent_id = %command.subagent_id,
            reason = %self.reason,
            "harness subagent delegation unavailable"
        );
        Ok(HarnessSubagentDelegationResult {
            task_id: None,
            final_message: None,
            events: Vec::new(),
            reason: Some(self.reason.clone()),
        })
    }
}

/// Null Object task repository for absent task services.
pub struct UnavailableHarnessSubagentTaskRepository {
    reason: String,
}

impl UnavailableHarnessSubagentTaskRepository {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HarnessSubagentTaskRepositoryPort for UnavailableHarnessSubagentTaskRepository {
    fn name(&self) -> &str {
        "unavailable_harness_subagent_task_repository"
    }

    async fn save(
        &self,
        runtime: RuntimeContext,
        record: HarnessSubagentTaskRecord,
    ) -> AgentResult<()> {
        warn!(
            trace_id = %runtime.trace_id,
            task_id = %record.task_id,
            reason = %self.reason,
            "harness subagent task save unavailable"
        );
        Ok(())
    }

    async fn get(
        &self,
        runtime: RuntimeContext,
        task_id: &str,
    ) -> AgentResult<Option<HarnessSubagentTaskRecord>> {
        warn!(
            trace_id = %runtime.trace_id,
            task_id,
            reason = %self.reason,
            "harness subagent task get unavailable"
        );
        Ok(None)
    }
}
