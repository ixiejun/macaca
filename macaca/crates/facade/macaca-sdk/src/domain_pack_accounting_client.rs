//! SDK helpers for the Finance Accounting domain pack.
//!
//! This module is a Facade plus Command helper. It performs provider-neutral
//! accounting preflight checks, then delegates command construction to the
//! generic [`DomainPackServiceCallBuilder`]. The SDK never constructs concrete
//! accounting providers, reads credentials, or applies application-specific
//! bookkeeping workflows.

use macaca_proto::domain_pack_contract::finance_accounting::{
    AccountingCommandPreflight, AccountingCommandPreflightSpec, AccountingPreflightRejection,
    FINANCE_ACCOUNTING_SERVICE_ID,
};
use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

/// Result of attempting to build an accounting command for the service runtime.
///
/// `Ready` carries the same canonical `ServiceCallCommand` produced by the
/// generic domain-pack builder. `Rejected` carries the typed accounting status
/// that service providers would return before touching a concrete adapter.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountingDomainPackCommandBuildOutcome {
    Ready(ServiceCallCommand),
    Rejected(AccountingPreflightRejection),
}

/// Builder for Finance Accounting service commands.
///
/// The builder is deliberately narrow: it owns accounting preflight evidence and
/// canonical command construction only. All service execution, provider Strategy
/// selection, policy engines, resource meters, and entitlement services remain
/// outside the SDK behind OS service boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountingDomainPackCommandBuilder {
    command_name: String,
    payload: serde_json::Value,
    preflight: AccountingCommandPreflight,
    trace: TraceContext,
}

impl AccountingDomainPackCommandBuilder {
    /// Create a builder for one accounting command.
    ///
    /// The command name must match the preflight command name so callers cannot
    /// accidentally approve one operation and dispatch another.
    pub fn new(
        command_name: impl Into<String>,
        payload: serde_json::Value,
        preflight: AccountingCommandPreflight,
        trace: TraceContext,
    ) -> MacacaResult<Self> {
        let command_name = command_name.into().trim().to_string();
        if command_name.is_empty() {
            return Err(MacacaError::Config(
                "accounting command builder requires a non-empty command name".into(),
            ));
        }
        if command_name != preflight.command_name {
            return Err(MacacaError::Config(
                "accounting command builder requires command and preflight names to match".into(),
            ));
        }
        Ok(Self {
            command_name,
            payload,
            preflight,
            trace,
        })
    }

    /// Evaluate accounting preflight and build the canonical service command.
    ///
    /// Rejections return typed accounting status without creating a service
    /// command, which proves the SDK helper cannot invoke a provider after a
    /// denied, unavailable, unsupported, quota, conflict, or stale-data decision.
    pub fn build(
        self,
        resolved: &DomainPackResolveResult,
    ) -> MacacaResult<AccountingDomainPackCommandBuildOutcome> {
        match AccountingCommandPreflightSpec.evaluate(&self.preflight) {
            Ok(()) => {
                info!(
                    service_id = FINANCE_ACCOUNTING_SERVICE_ID,
                    command = %self.command_name,
                    trace_id = %self.trace.trace_id,
                    "accounting_pack_preflight_allowed"
                );
                let command = DomainPackServiceCallBuilder::new(
                    FINANCE_ACCOUNTING_SERVICE_ID,
                    self.command_name,
                    self.payload,
                    self.trace,
                )?
                .build(resolved)?;
                Ok(AccountingDomainPackCommandBuildOutcome::Ready(command))
            }
            Err(rejection) => {
                warn!(
                    service_id = FINANCE_ACCOUNTING_SERVICE_ID,
                    command = %self.command_name,
                    trace_id = %self.trace.trace_id,
                    status = ?rejection.status,
                    reason_code = %rejection.reason_code,
                    "accounting_pack_preflight_rejected"
                );
                Ok(AccountingDomainPackCommandBuildOutcome::Rejected(rejection))
            }
        }
    }
}

#[cfg(test)]
#[path = "domain_pack_accounting_client_tests.rs"]
mod tests;
