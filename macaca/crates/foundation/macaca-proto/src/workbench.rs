//! Provider-neutral contracts for Codex-class interactive application support.
//!
//! The workbench contracts define the generic substrate needed by coding,
//! migration, documentation, QA, compliance, and other interactive agent
//! applications.  They intentionally do not encode a product workflow.  Every
//! command is wrapped with application/session scope and [`TraceContext`] so
//! service runtimes can apply policy, append sanitized audit evidence, and
//! route to replaceable providers before any side effect.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ApplicationId, CapabilityId, KernelServiceId, MacacaError, MacacaResult, ServiceCapability,
    ServiceDescriptor, ServiceHealth, ServiceLifecycleState, ServiceScope, ServiceType,
    TraceContext, TraceSchemaRef,
};

pub mod app_protocol;
pub mod approval;
pub mod code_intelligence;
pub mod config;
pub mod diagnostics;
pub mod file;
pub mod git;
pub mod hook;
pub mod interaction;
pub mod plugin_marketplace;
pub mod process;
pub mod realtime;
pub mod remote_environment;
pub mod review;
pub mod sandbox;

/// Compatibility module name used by the initial workbench descriptor catalog.
///
/// The typed contracts now live in [`config`], but `config_service` remains as a
/// stable import path for focused SDK clients and shell adapters that were
/// introduced before the full `service.config` provider landed.
pub mod config_service {
    pub use super::config::*;
}

/// Stable command outcomes shared by all workbench-family services.
///
/// The enum is deliberately data-only so absent providers, policy denials, and
/// timeout/cancellation paths are explicit protocol states rather than panics,
/// fake success, or provider-specific exceptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkbenchCommandStatus {
    Unavailable { reason: String },
    Unsupported { reason: String },
    Denied { reason: String },
    Failed { reason: String },
    PendingApproval { approval_ref: String },
    Cancelled { reason: String },
    Timeout { reason: String },
    ArtifactBacked { artifact_ref: ArtifactRef },
    Completed,
}

impl WorkbenchCommandStatus {
    /// Build the standard Null Object unavailable state used by SDK clients.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }
}

/// Reference to payload material stored outside inline protocol responses.
///
/// Services use artifact refs when file contents, process output, provider
/// payloads, diagnostics bundles, or review evidence exceed inline budgets or
/// require stricter redaction.  The ref is safe to log; the material it points
/// to is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: String,
    pub media_type: String,
    pub sha256: Option<String>,
    pub byte_len: u64,
    pub redaction_profile: String,
}

/// Bounded inline summary for large or sensitive payload families.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedSummary {
    pub text: String,
    pub truncated: bool,
    pub original_byte_len: Option<u64>,
    pub artifact_ref: Option<ArtifactRef>,
}

/// Shared command scope required before any service dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchCommandScope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_ref: Option<String>,
    pub thread_ref: Option<String>,
    pub turn_ref: Option<String>,
}

impl WorkbenchCommandScope {
    /// Create a command scope and fail fast when session identity is missing.
    pub fn new(application_id: ApplicationId, session_id: impl Into<String>) -> MacacaResult<Self> {
        let session_id = session_id.into().trim().to_string();
        if session_id.is_empty() {
            return Err(MacacaError::Config(
                "workbench command scope requires non-empty session_id".into(),
            ));
        }
        Ok(Self {
            application_id,
            session_id,
            task_ref: None,
            thread_ref: None,
            turn_ref: None,
        })
    }
}

/// Trace-required command envelope used by all workbench service clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchCommand<T> {
    pub scope: WorkbenchCommandScope,
    pub trace: TraceContext,
    pub command_id: String,
    pub payload: T,
    pub metadata: BTreeMap<String, String>,
}

impl<T> WorkbenchCommand<T> {
    /// Build a command envelope with an explicit trace and provider-neutral
    /// payload.  Logging should include command_id/trace only, never raw
    /// payload material unless a service-specific redaction strategy permits it.
    pub fn new(
        scope: WorkbenchCommandScope,
        trace: TraceContext,
        command_id: impl Into<String>,
        payload: T,
    ) -> MacacaResult<Self> {
        let command_id = command_id.into().trim().to_string();
        if command_id.is_empty() {
            return Err(MacacaError::Config(
                "workbench command requires non-empty command_id".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            command_id,
            payload,
            metadata: BTreeMap::new(),
        })
    }
}

/// Shared result envelope for focused SDK clients and service providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchCommandResult<T> {
    pub trace: TraceContext,
    pub status: WorkbenchCommandStatus,
    pub output: Option<T>,
    pub summary: Option<BoundedSummary>,
    pub audit_ref: Option<String>,
    pub completed_at: DateTime<Utc>,
}

impl<T> WorkbenchCommandResult<T> {
    /// Build a structured unavailable result for Null Object providers.
    pub fn unavailable(trace: TraceContext, reason: impl Into<String>) -> Self {
        Self {
            trace,
            status: WorkbenchCommandStatus::unavailable(reason),
            output: None,
            summary: None,
            audit_ref: None,
            completed_at: Utc::now(),
        }
    }
}

/// Descriptor extension required by Codex-class workbench services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchServiceDescriptor {
    pub base: ServiceDescriptor,
    pub commands: Vec<&'static str>,
    pub event_schema: &'static str,
    pub audit_schema: &'static str,
    pub snapshot_schema: &'static str,
}

impl WorkbenchServiceDescriptor {
    /// Build a descriptor from stable service metadata.
    pub fn new(
        service_id: &'static str,
        capability_id: &'static str,
        commands: Vec<&'static str>,
        event_schema: &'static str,
        audit_schema: &'static str,
        snapshot_schema: &'static str,
    ) -> Self {
        let mut base = ServiceDescriptor::new(
            KernelServiceId::new(service_id),
            ServiceType::new("interactive.workbench"),
            TraceSchemaRef::new(format!("trace.{service_id}.v1")),
        );
        base.capabilities.push(ServiceCapability::new(
            CapabilityId::new(capability_id),
            format!("{service_id} workbench capability"),
        ));
        base.lifecycle_state = ServiceLifecycleState::Registered;
        base.health = ServiceHealth::Unavailable {
            reason: "provider not installed".into(),
        };
        base.supported_scopes = vec![ServiceScope::Global];
        Self {
            base,
            commands,
            event_schema,
            audit_schema,
            snapshot_schema,
        }
    }
}

/// Return descriptors for every workbench-family service introduced by the
/// foundation contract slice.
pub fn workbench_service_descriptors() -> Vec<WorkbenchServiceDescriptor> {
    vec![
        interaction::descriptor(),
        app_protocol::descriptor(),
        file::descriptor(),
        process::descriptor(),
        sandbox::descriptor(),
        approval::descriptor(),
        hook::descriptor(),
        config_service::descriptor(),
        plugin_marketplace::descriptor(),
        code_intelligence::descriptor(),
        git::descriptor(),
        review::descriptor(),
        diagnostics::descriptor(),
        realtime::descriptor(),
        remote_environment::descriptor(),
    ]
}
