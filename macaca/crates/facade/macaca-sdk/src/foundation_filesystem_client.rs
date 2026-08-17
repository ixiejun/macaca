//! SDK Facade helpers for `pack.foundation.filesystem.v1`.
//!
//! These helpers create only canonical traced service calls. They do not resolve
//! logical roots, access host paths, read content, or construct filesystem
//! providers; those concerns remain behind the service runtime boundary.

use macaca_proto::{
    FilesystemAdmissionFailure, FilesystemResourceReservation, MacacaResult, TraceContext,
};
use tracing::{info, warn};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

const SERVICE_ID: &str = "service.foundation.filesystem";

/// Result of filesystem preflight and canonical command construction.
#[derive(Debug, Clone, PartialEq)]
pub enum FilesystemDomainPackCommandBuildOutcome {
    Ready(ServiceCallCommand),
    Rejected(FilesystemAdmissionFailure),
}

/// Provider-neutral builder for a declared foundation filesystem command.
#[derive(Debug, Clone, PartialEq)]
pub struct FilesystemDomainPackCommandBuilder {
    command_name: String,
    payload: serde_json::Value,
    trace: TraceContext,
}

impl FilesystemDomainPackCommandBuilder {
    /// Capture a validated DTO payload and its required trace context.
    pub fn new(
        command_name: impl Into<String>,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> Self {
        Self {
            command_name: command_name.into(),
            payload,
            trace,
        }
    }

    /// Build an admitted generic service call without exposing provider state.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        info!(service_id = SERVICE_ID, command = %self.command_name, trace_id = %self.trace.trace_id,
            "foundation_filesystem_sdk_command_built");
        DomainPackServiceCallBuilder::new(SERVICE_ID, self.command_name, self.payload, self.trace)?
            .build(resolved)
    }

    /// Build only after admission has reserved resources and approved the request.
    pub fn build_after_preflight(
        self,
        resolved: &DomainPackResolveResult,
        decision: Result<FilesystemResourceReservation, FilesystemAdmissionFailure>,
    ) -> MacacaResult<FilesystemDomainPackCommandBuildOutcome> {
        match decision {
            Ok(_) => Ok(FilesystemDomainPackCommandBuildOutcome::Ready(
                self.build(resolved)?,
            )),
            Err(reason) => {
                warn!(service_id = SERVICE_ID, trace_id = %self.trace.trace_id,
                    status = ?reason, "foundation_filesystem_sdk_preflight_rejected");
                Ok(FilesystemDomainPackCommandBuildOutcome::Rejected(reason))
            }
        }
    }
}

macro_rules! filesystem_command {
    ($function:ident, $command:literal) => {
        #[doc = concat!("Build `", $command, "` through the filesystem service runtime.")]
        pub fn $function(
            payload: serde_json::Value,
            trace: TraceContext,
        ) -> FilesystemDomainPackCommandBuilder {
            FilesystemDomainPackCommandBuilder::new($command, payload, trace)
        }
    };
}

filesystem_command!(filesystem_open_handle_command, "filesystem.open_handle");
filesystem_command!(filesystem_close_handle_command, "filesystem.close_handle");
filesystem_command!(filesystem_read_file_command, "filesystem.read_file");
filesystem_command!(filesystem_write_file_command, "filesystem.write_file");
filesystem_command!(filesystem_append_file_command, "filesystem.append_file");
filesystem_command!(
    filesystem_list_directory_command,
    "filesystem.list_directory"
);
filesystem_command!(filesystem_stat_path_command, "filesystem.stat_path");
filesystem_command!(
    filesystem_create_directory_command,
    "filesystem.create_directory"
);
filesystem_command!(filesystem_copy_path_command, "filesystem.copy_path");
filesystem_command!(filesystem_move_path_command, "filesystem.move_path");
filesystem_command!(filesystem_delete_path_command, "filesystem.delete_path");
filesystem_command!(filesystem_create_temp_command, "filesystem.create_temp");
filesystem_command!(filesystem_watch_path_command, "filesystem.watch_path");
filesystem_command!(filesystem_snapshot_tree_command, "filesystem.snapshot_tree");
filesystem_command!(
    filesystem_restore_snapshot_command,
    "filesystem.restore_snapshot"
);

#[cfg(test)]
#[path = "foundation_filesystem_client_tests.rs"]
mod tests;
