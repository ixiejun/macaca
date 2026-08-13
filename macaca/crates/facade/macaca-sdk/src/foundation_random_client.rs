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

#[cfg(test)]
#[path = "foundation_random_client_tests.rs"]
mod tests;
