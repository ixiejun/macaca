//! Contract tests for the built-in Skill system service provider.
//!
//! These tests prove Skill service commands stay provider-neutral,
//! trace-friendly, and bounded while governance, curation, alias, and evolution
//! flows remain application-agnostic.
//!
//! Module layout (Contract Test + Object Mother + Test Double):
//! - `fixtures.rs` — traced command builders and reusable observation/candidate mothers
//! - `test_doubles.rs` — recording Memory runtime facade for destination routing proofs
//! - `governance_evaluation_tests.rs` — governance read model, status telemetry, evaluation
//! - `curation_dry_run_tests.rs` — dry-run curation without filesystem mutation
//! - `curation_run_snapshot_tests.rs` — curation run, snapshot, and report sanitization
//! - `curation_rollback_tests.rs` — rollback memento restore and telemetry dry-run counters
//! - `alias_tests.rs` — alias upsert/resolve/snapshot and resolution policy statuses
//! - `alias_helpers.rs` — shared alias policy test helpers
//! - `experience_proposal_tests.rs` — draft proposal creation and snapshot listing
//! - `experience_routing_tests.rs` — memory/knowledge destination routing through facades
//! - `experience_validation_tests.rs` — candidate validation rejection paths
//! - `governance_replay_tests.rs` — event journal replay read model without skill bodies

mod alias_helpers;
mod alias_tests;
mod curation_dry_run_tests;
mod curation_rollback_tests;
mod curation_run_snapshot_tests;
mod experience_proposal_tests;
mod experience_routing_tests;
mod experience_validation_tests;
mod fixtures;
mod governance_evaluation_tests;
mod governance_replay_tests;
mod test_doubles;
