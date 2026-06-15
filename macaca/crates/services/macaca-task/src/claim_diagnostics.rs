//! Re-exported Task Board claim diagnostics.
//!
//! The diagnostic algorithm is provider-neutral and operates only on
//! `macaca-proto` task DTOs, so `macaca-proto` owns the canonical
//! implementation. This module keeps the task-service internal path stable
//! while avoiding duplicate projection logic.

pub use macaca_proto::{
    diagnose_session_claims, AgentClaimReport, ClaimGate, DependencySnap, SessionClaimDiagnostics,
};
