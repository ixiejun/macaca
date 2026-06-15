//! Snapshot payload decoding for the plugin marketplace service.
//!
//! Workbench diagnostics send a generic `WorkbenchCommand<Value>` envelope for
//! every service, while marketplace-specific callers may still send the typed
//! snapshot command directly.  This Adapter normalizes both shapes into one
//! `PluginMarketplaceSnapshotCommand` so the Command handler stays typed and
//! the generic diagnostics path remains stable.

use macaca_proto::workbench::plugin_marketplace::PluginMarketplaceSnapshotCommand;
use macaca_proto::{ServiceResult, WorkbenchCommand};
use serde_json::Value;
use tracing::debug;

use crate::plugin_marketplace_service_support::decode;

/// Decode a marketplace snapshot command from either a typed DTO or a workbench envelope.
pub(crate) fn decode_snapshot_payload(
    value: Value,
) -> ServiceResult<PluginMarketplaceSnapshotCommand> {
    if value.is_null() {
        debug!(
            "plugin marketplace snapshot decode: null payload defaults to empty snapshot command"
        );
        return Ok(PluginMarketplaceSnapshotCommand {
            include_audit_tail: false,
        });
    }
    if let Ok(command) = serde_json::from_value::<PluginMarketplaceSnapshotCommand>(value.clone()) {
        debug!(
            include_audit_tail = command.include_audit_tail,
            "plugin marketplace snapshot decode: accepted typed snapshot command"
        );
        return Ok(command);
    }
    if let Ok(envelope) = serde_json::from_value::<WorkbenchCommand<Value>>(value.clone()) {
        if envelope.payload.is_null() {
            debug!(
                command_id = %envelope.command_id,
                "plugin marketplace snapshot decode: workbench envelope carried null payload"
            );
            return Ok(PluginMarketplaceSnapshotCommand {
                include_audit_tail: false,
            });
        }
        debug!(
            command_id = %envelope.command_id,
            "plugin marketplace snapshot decode: unwrapping workbench envelope payload"
        );
        return decode(envelope.payload);
    }
    debug!("plugin marketplace snapshot decode: falling back to direct typed decode");
    decode(value)
}
