//! SDK Facade helpers for `pack.foundation.random.v1`.
//!
//! These helpers validate already-computed policy evidence and build canonical
//! traced service calls. They never access host entropy or construct providers.

use macaca_proto::domain_pack_contract::foundation_random_semantics::{
    RandomAdmissionFailure, RandomResourceReservation,
};
use macaca_proto::{MacacaResult, TraceContext};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

const SERVICE_ID: &str = "service.foundation.random";

/// Result of random SDK preflight and command construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RandomDomainPackCommandBuildOutcome {
    Ready(ServiceCallCommand),
    Rejected(RandomAdmissionFailure),
}

/// Provider-neutral random command Facade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomDomainPackCommandBuilder {
    command_name: String,
    payload: serde_json::Value,
    decision: Result<RandomResourceReservation, RandomAdmissionFailure>,
    trace: TraceContext,
}

impl RandomDomainPackCommandBuilder {
    /// Create a builder carrying an already evaluated service policy decision.
    pub fn new(
        command_name: impl Into<String>,
        payload: serde_json::Value,
        decision: Result<RandomResourceReservation, RandomAdmissionFailure>,
        trace: TraceContext,
    ) -> Self {
        Self {
            command_name: command_name.into(),
            payload,
            decision,
            trace,
        }
    }

    /// Build only an admitted canonical service call; rejected calls stop here.
    pub fn build(
        self,
        resolved: &DomainPackResolveResult,
    ) -> MacacaResult<RandomDomainPackCommandBuildOutcome> {
        match self.decision {
            Ok(_) => {
                info!(service_id = SERVICE_ID, command = %self.command_name, trace_id = %self.trace.trace_id, "random_pack_sdk_preflight_allowed");
                Ok(RandomDomainPackCommandBuildOutcome::Ready(
                    DomainPackServiceCallBuilder::new(
                        SERVICE_ID,
                        self.command_name,
                        self.payload,
                        self.trace,
                    )?
                    .build(resolved)?,
                ))
            }
            Err(reason) => {
                warn!(service_id = SERVICE_ID, trace_id = %self.trace.trace_id, status = ?reason, "random_pack_sdk_preflight_rejected");
                Ok(RandomDomainPackCommandBuildOutcome::Rejected(reason))
            }
        }
    }
}

/// Build a typed byte request through the generic descriptor builder.
pub fn random_bytes_command(
    payload: serde_json::Value,
    decision: Result<RandomResourceReservation, RandomAdmissionFailure>,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    RandomDomainPackCommandBuilder::new("random.bytes", payload, decision, trace)
}

/// Build an identifier command through the canonical service boundary.
pub fn random_uuid_v4_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    read_only_command("random.uuid_v4", payload, trace)
}

/// Build a nonce command through the canonical service boundary.
pub fn random_nonce_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    read_only_command("random.nonce", payload, trace)
}

/// Build a token command after caller-owned policy preflight.
pub fn random_token_command(
    payload: serde_json::Value,
    decision: Result<RandomResourceReservation, RandomAdmissionFailure>,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    RandomDomainPackCommandBuilder::new("random.token", payload, decision, trace)
}

/// Build a bias-free integer request through the canonical service boundary.
pub fn random_integer_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    read_only_command("random.integer", payload, trace)
}

/// Build a deterministic stream command only after explicit test-context admission.
pub fn random_test_stream_command(
    payload: serde_json::Value,
    decision: Result<RandomResourceReservation, RandomAdmissionFailure>,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    RandomDomainPackCommandBuilder::new("random.test_stream_create", payload, decision, trace)
}

/// Build a bounded entropy diagnostic query without exposing provider handles.
pub fn random_entropy_health_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    read_only_command("random.entropy_health", payload, trace)
}

/// Build a provider capability query without exposing native RNG state.
pub fn random_provider_capabilities_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    read_only_command("random.provider_capabilities", payload, trace)
}

/// Centralize the zero-cost reservation used by read-only or bounded commands.
fn read_only_command(
    command_name: &str,
    payload: serde_json::Value,
    trace: TraceContext,
) -> RandomDomainPackCommandBuilder {
    RandomDomainPackCommandBuilder::new(
        command_name,
        payload,
        Ok(RandomResourceReservation {
            request_units: 1,
            ..Default::default()
        }),
        trace,
    )
}

#[cfg(test)]
#[path = "foundation_random_client_tests.rs"]
mod tests;
