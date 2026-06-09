//! Migration-debt allowlist for OS-layer files exceeding the 500-line constitution.
//!
//! Each row records a known oversized production `src/**/*.rs` file. The executable
//! gate rejects any NEW violation while this baseline converges toward zero rows.
//! Sync removals with OpenSpec task §4.5.1 / §6.1.6 when splitting modules.

use super::gate::FileSizeAllowlistEntry;

/// Returns the baseline oversized-file snapshot (generated 2026-06-08, iteration 112).
///
/// P4 file-size gate complete: zero allowlist rows.
pub fn entries() -> Vec<FileSizeAllowlistEntry> {
    vec![]
}
