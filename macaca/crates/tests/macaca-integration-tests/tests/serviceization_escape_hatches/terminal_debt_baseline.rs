//! Frozen terminal-debt inventory for serviceization escape-hatch tokens (P5 §6.2).
//!
//! When `honor_terminal_exception_surfaces` is false, the scanner reports **raw** violations —
//! every forbidden token hit in production `src/` regardless of approved terminal
//! exception surfaces. This module freezes that inventory so CI fails if debt grows **or**
//! shrinks without an explicit baseline update (Strangler Fig retirement).
//!
//! Update this file only when OpenSpec tasks document a deliberate terminal exception surface
//! removal or a new frozen debt row. Regenerate family counts with:
//! `cargo test -p macaca-integration-tests dump_escape_hatch_raw_fingerprints -- --ignored --nocapture`

/// Total raw hits when terminal exception surfaces are not honored (iteration 54 snapshot).
///
/// `provider-model-routing-name` family retired (−121): `macaca-llm/src/` is the
/// canonical routing owner (exempt from raw inventory); all other production layers
/// shed vendor/model literals, comment-only doc examples, and config defaults now
/// resolve from user manifests or `config/default.toml`.
pub const EXPECTED_RAW_VIOLATION_COUNT: usize = 0;

/// Per-family raw hit counts — detects debt shifting between token families.
pub const EXPECTED_RAW_VIOLATION_BY_FAMILY: &[(&str, usize)] = &[];
