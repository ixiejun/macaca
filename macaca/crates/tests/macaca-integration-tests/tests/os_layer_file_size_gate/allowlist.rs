//! Migration-debt allowlist for OS-layer files exceeding the 500-line constitution.
//!
//! Each row records a known oversized production `src/**/*.rs` file. The executable
//! gate rejects any NEW violation while this baseline converges toward zero rows.
//! Sync removals with OpenSpec task §4.5.1 / §6.1.6 when splitting modules.

use super::gate::FileSizeAllowlistEntry;

/// Returns the baseline oversized-file snapshot (generated 2026-06-08, iteration 104).
pub fn entries() -> Vec<FileSizeAllowlistEntry> {
    vec![
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/a2a.rs", 873, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/formatter.rs", 1354, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/mcp.rs", 1750, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/memory.rs", 1303, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/model_impls.rs", 893, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/plan.rs", 934, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/react_agent.rs", 1040, "P4-framework", "P4"),
        FileSizeAllowlistEntry::new("crates/runtime/macaca-framework/src/tool.rs", 1042, "P4-framework", "P4"),
    ]
}
