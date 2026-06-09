// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Harness filesystem and sandbox service ports.
//!
//! Framework Harness must not read or write host paths directly. These ports
//! define command/result DTOs for runtime-host driver/filesystem/sandbox
//! services, where path policy, entitlement, resource, and audit decorators can
//! run before privileged side effects.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::agent::AgentResult;
use crate::runtime_context::RuntimeContext;

/// Provider-neutral filesystem operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemOperation {
    Read,
    Write,
    Delete,
    Move,
    Exists,
    Glob,
}

/// Command sent to a filesystem service adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessFilesystemCommand {
    pub runtime: RuntimeContext,
    pub operation: FilesystemOperation,
    pub path: String,
    pub destination_path: Option<String>,
    pub content: Option<String>,
}

/// Structured filesystem result. `denied` and `unavailable` are explicit
/// states so callers never need to infer failure from empty content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessFilesystemResult {
    pub success: bool,
    pub content: Option<String>,
    pub paths: Vec<String>,
    pub reason: Option<String>,
    pub evidence_ref: Option<String>,
}

/// Replaceable filesystem boundary for Harness tools and middleware.
#[async_trait]
pub trait HarnessFilesystemPort: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(
        &self,
        command: HarnessFilesystemCommand,
    ) -> AgentResult<HarnessFilesystemResult>;
}

/// Null Object filesystem port for absent or disabled providers.
pub struct UnavailableHarnessFilesystemPort {
    reason: String,
}

impl UnavailableHarnessFilesystemPort {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HarnessFilesystemPort for UnavailableHarnessFilesystemPort {
    fn name(&self) -> &str {
        "unavailable_harness_filesystem"
    }

    async fn execute(
        &self,
        command: HarnessFilesystemCommand,
    ) -> AgentResult<HarnessFilesystemResult> {
        warn!(
            trace_id = %command.runtime.trace_id,
            operation = ?command.operation,
            path = %command.path,
            reason = %self.reason,
            "harness filesystem operation unavailable"
        );
        Ok(HarnessFilesystemResult {
            success: false,
            content: None,
            paths: Vec::new(),
            reason: Some(self.reason.clone()),
            evidence_ref: None,
        })
    }
}

/// Command for acquiring, releasing, or snapshotting a sandbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessSandboxCommand {
    pub runtime: RuntimeContext,
    pub workspace_ref: Option<String>,
    pub isolation_key: String,
}

/// Lease returned by a sandbox service after policy/resource admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSandboxLease {
    pub lease_id: String,
    pub isolation_key: String,
    pub evidence_ref: Option<String>,
}

/// Bounded sandbox snapshot metadata. Raw files, package bytes, and provider
/// payloads must stay outside this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSandboxSnapshot {
    pub lease_id: Option<String>,
    pub available: bool,
    pub reason: Option<String>,
    pub evidence_ref: Option<String>,
}

/// Replaceable sandbox service port.
#[async_trait]
pub trait HarnessSandboxPort: Send + Sync {
    fn name(&self) -> &str;
    async fn acquire(
        &self,
        command: HarnessSandboxCommand,
    ) -> AgentResult<Option<HarnessSandboxLease>>;
    async fn release(
        &self,
        command: HarnessSandboxCommand,
        lease_id: &str,
    ) -> AgentResult<HarnessSandboxSnapshot>;
    async fn snapshot(&self, command: HarnessSandboxCommand)
        -> AgentResult<HarnessSandboxSnapshot>;
}

/// Null Object sandbox port for absent optional sandbox modules.
pub struct UnavailableHarnessSandboxPort {
    reason: String,
}

impl UnavailableHarnessSandboxPort {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HarnessSandboxPort for UnavailableHarnessSandboxPort {
    fn name(&self) -> &str {
        "unavailable_harness_sandbox"
    }

    async fn acquire(
        &self,
        command: HarnessSandboxCommand,
    ) -> AgentResult<Option<HarnessSandboxLease>> {
        warn!(
            trace_id = %command.runtime.trace_id,
            isolation_key = %command.isolation_key,
            reason = %self.reason,
            "harness sandbox acquire unavailable"
        );
        Ok(None)
    }

    async fn release(
        &self,
        command: HarnessSandboxCommand,
        lease_id: &str,
    ) -> AgentResult<HarnessSandboxSnapshot> {
        warn!(
            trace_id = %command.runtime.trace_id,
            isolation_key = %command.isolation_key,
            lease_id,
            reason = %self.reason,
            "harness sandbox release unavailable"
        );
        Ok(unavailable_snapshot(&self.reason))
    }

    async fn snapshot(
        &self,
        command: HarnessSandboxCommand,
    ) -> AgentResult<HarnessSandboxSnapshot> {
        warn!(
            trace_id = %command.runtime.trace_id,
            isolation_key = %command.isolation_key,
            reason = %self.reason,
            "harness sandbox snapshot unavailable"
        );
        Ok(unavailable_snapshot(&self.reason))
    }
}

fn unavailable_snapshot(reason: &str) -> HarnessSandboxSnapshot {
    HarnessSandboxSnapshot {
        lease_id: None,
        available: false,
        reason: Some(reason.to_string()),
        evidence_ref: None,
    }
}
