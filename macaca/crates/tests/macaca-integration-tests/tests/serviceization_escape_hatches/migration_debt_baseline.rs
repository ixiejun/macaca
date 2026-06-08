//! Frozen migration-debt inventory for serviceization escape-hatch tokens (P5 §6.2).
//!
//! When `honor_migration_surfaces` is false, the scanner reports **raw** violations —
//! every forbidden token hit in production `src/` regardless of approved migration
//! surfaces. This module freezes that inventory so CI fails if debt grows **or**
//! shrinks without an explicit baseline update (Strangler Fig retirement).
//!
//! Update this file only when OpenSpec tasks document a deliberate migration surface
//! removal or a new frozen debt row. Regenerate family counts with:
//! `cargo test -p macaca-integration-tests dump_escape_hatch_raw_fingerprints -- --ignored --nocapture`

/// Total raw hits when migration surfaces are not honored (iteration 49 snapshot).
///
/// `autonomy-service-boundary` sub-phase (−23): `SchedulerSystemServiceProvider` /
/// `HeartbeatSystemServiceProvider` → `Host*ServiceAdapter`; `AutonomySupervisor` →
/// `AutonomyLifecycleCoordinator`; service id literals → `SCHEDULER_SERVICE_ID` /
/// `HEARTBEAT_SERVICE_ID` proto constants.
pub const EXPECTED_RAW_VIOLATION_COUNT: usize = 177;

/// Per-family raw hit counts — detects debt shifting between token families.
pub const EXPECTED_RAW_VIOLATION_BY_FAMILY: &[(&str, usize)] = &[
    ("autonomy-service-boundary", 2),
    ("hardcoded-agent-role", 54),
    ("provider-model-routing-name", 121),
];
