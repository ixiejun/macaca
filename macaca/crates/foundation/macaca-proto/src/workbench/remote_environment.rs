//! Optional remote execution environment service contracts.
//!
//! Remote environments are registered through a generic service boundary so
//! applications can select remote execution without embedding SSH, container,
//! or vendor-specific behavior in the OS.  Provider absence is a normal
//! structured state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ServiceHealth;

use super::{BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.remote_environment";
pub const CAPABILITY_ID: &str = "capability.remote_environment";
pub const ADD_COMMAND: &str = "remote_environment.add";
pub const HEALTH_COMMAND: &str = "remote_environment.health";
pub const WORKSPACE_ROOTS_COMMAND: &str = "remote_environment.workspace_roots";
pub const CLEANUP_COMMAND: &str = "remote_environment.cleanup";
pub const SELECT_COMMAND: &str = "remote_environment.select";
pub const REMOVE_COMMAND: &str = "remote_environment.remove";

pub const COMMANDS: &[&str] = &[
    ADD_COMMAND,
    HEALTH_COMMAND,
    WORKSPACE_ROOTS_COMMAND,
    CLEANUP_COMMAND,
    SELECT_COMMAND,
    REMOVE_COMMAND,
];

pub type RemoteEnvironmentCommand<T> = WorkbenchCommand<T>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteEnvironmentProviderKind {
    RemoteExecServer,
    Container,
    VirtualMachine,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteEnvironmentStatus {
    Registered,
    Selected,
    Unhealthy,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEnvironmentAddRequest {
    pub environment_ref: Option<String>,
    pub provider_kind: RemoteEnvironmentProviderKind,
    pub endpoint_ref: String,
    pub workspace_roots: Vec<String>,
    pub connection_summary: BoundedSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEnvironmentRefRequest {
    pub environment_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEnvironmentSelectRequest {
    pub environment_ref: String,
    pub reason: Option<BoundedSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEnvironmentRecord {
    pub environment_ref: String,
    pub provider_kind: RemoteEnvironmentProviderKind,
    pub endpoint_ref: String,
    pub workspace_roots: Vec<String>,
    pub status: RemoteEnvironmentStatus,
    pub health: ServiceHealth,
    pub connection_summary: BoundedSummary,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteEnvironmentServiceResponse {
    Environment(RemoteEnvironmentRecord),
    Snapshot(Vec<RemoteEnvironmentRecord>),
    Health(RemoteEnvironmentRecord),
    WorkspaceRoots {
        environment_ref: String,
        workspace_roots: Vec<String>,
    },
    Cleanup {
        environment_ref: String,
        cleaned: bool,
    },
    Removed {
        environment_ref: String,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.remote_environment.events.v1",
        "service.remote_environment.audit.v1",
        "service.remote_environment.snapshot.v1",
    )
}
