//! Root-export bucket for the foundation filesystem contract.
//!
//! This keeps the compatibility re-exports available through `macaca_proto::*`
//! while keeping the aggregate export module under the source-size limit.

pub use super::foundation_filesystem::*;
pub use super::foundation_filesystem_semantics::{
    dispatch_after_preflight as dispatch_filesystem_after_preflight,
    preflight_command as preflight_filesystem_command, redacted_filesystem_audit_fields,
    reserve as reserve_filesystem_resources, FilesystemAdmissionFailure,
    FilesystemAdmissionRequest, FilesystemAuditFields, FilesystemPolicyContext,
    FilesystemResourceLimits, FilesystemResourceReservation,
};
pub use super::foundation_filesystem_validation::validate_filesystem_root_declarations;
