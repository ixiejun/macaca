//! Optional Web3 service facade for Route C Phase 10.
//!
//! The facade composes Null Object, Adapter, Strategy, Facade, and Observer
//! patterns. Kernel code coordinates availability, policy, trace, and adapter
//! boundaries, but it never implements a concrete chain, wallet, node, RPC, or
//! private-key provider. Missing Web3 is represented as structured data so the
//! base OS remains available without Web3 installed.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tracing::{info, warn};

use macaca_proto::{
    ChainId, ChainQueryRequest, ChainQueryResponse, SigningDecision, SigningPolicy, SigningProof,
    SigningRequest, TransactionReceipt, TransactionRequest, Web3Availability, Web3CapabilityKind,
    Web3Error, Web3ReceiptId, Web3RequestId, Web3TransactionId, Web3UnavailableReason,
};

use crate::web3_event::{NoopWeb3TraceEventSink, Web3TraceEvent, Web3TraceEventSink};

/// Policy strategy for signing, transaction, and chain-query requests.
pub trait Web3PolicyEngine: Send + Sync {
    /// Evaluate a request against availability, explicit approval, and region
    /// constraints before an adapter is allowed to execute.
    fn evaluate(
        &self,
        request_id: &Web3RequestId,
        availability: &Web3Availability,
        policy: &SigningPolicy,
        operation: &str,
    ) -> Result<SigningDecision, Web3Error>;
}

/// Conservative default policy used by the optional facade.
#[derive(Debug, Default, Clone)]
pub struct DefaultWeb3PolicyEngine;

impl DefaultWeb3PolicyEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Web3PolicyEngine for DefaultWeb3PolicyEngine {
    fn evaluate(
        &self,
        request_id: &Web3RequestId,
        availability: &Web3Availability,
        policy: &SigningPolicy,
        operation: &str,
    ) -> Result<SigningDecision, Web3Error> {
        info!(
            request_id = %request_id,
            operation,
            availability = %availability.state,
            "web3 policy evaluation started"
        );
        if availability.state == "region_blocked" || !policy.region_allowed {
            return Err(Web3Error::RegionBlocked(
                "web3 request blocked by region policy".into(),
            ));
        }
        if availability.state == "disabled_by_policy" || !policy.module_enabled {
            return Err(Web3Error::DisabledByPolicy(
                "web3 module disabled by policy".into(),
            ));
        }
        if !availability.is_available() {
            return Err(Web3Error::Unavailable(
                "web3 optional module is unavailable".into(),
            ));
        }
        if policy.explicit_approval_required && !policy.explicit_approval_granted {
            return Err(Web3Error::ApprovalRequired(
                "explicit approval required before web3 execution".into(),
            ));
        }
        Ok(SigningDecision {
            request_id: request_id.clone(),
            allowed: true,
            reason: format!("web3 {operation} approved"),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        })
    }
}

/// Replaceable Web3 adapter boundary.
#[async_trait]
pub trait Web3Adapter: Send + Sync {
    /// Return adapter availability and supported capabilities.
    async fn availability(&self) -> Web3Availability;

    /// Sign a bounded digest after policy approval.
    async fn sign(
        &self,
        request: &SigningRequest,
        decision: &SigningDecision,
    ) -> Result<SigningProof, Web3Error>;

    /// Submit or simulate a transaction after policy approval.
    async fn submit_transaction(
        &self,
        request: &TransactionRequest,
        decision: &SigningDecision,
    ) -> Result<TransactionReceipt, Web3Error>;

    /// Query chain state through a provider-neutral command.
    async fn query_chain(
        &self,
        request: &ChainQueryRequest,
    ) -> Result<ChainQueryResponse, Web3Error>;
}

/// Null Object adapter used when Web3 is not installed.
#[deprecated(
    note = "Use Web3SystemServiceProvider plus SystemWeb3Client for new Web3 service paths"
)]
#[derive(Debug, Default)]
pub struct UnavailableWeb3Adapter;

#[async_trait]
impl Web3Adapter for UnavailableWeb3Adapter {
    async fn availability(&self) -> Web3Availability {
        Web3Availability::unavailable(Web3UnavailableReason::new(
            "not_installed",
            "optional web3 module is not installed",
        ))
    }

    async fn sign(
        &self,
        _request: &SigningRequest,
        _decision: &SigningDecision,
    ) -> Result<SigningProof, Web3Error> {
        Err(Web3Error::Unavailable(
            "optional web3 signing adapter is unavailable".into(),
        ))
    }

    async fn submit_transaction(
        &self,
        _request: &TransactionRequest,
        _decision: &SigningDecision,
    ) -> Result<TransactionReceipt, Web3Error> {
        Err(Web3Error::Unavailable(
            "optional web3 transaction adapter is unavailable".into(),
        ))
    }

    async fn query_chain(
        &self,
        _request: &ChainQueryRequest,
    ) -> Result<ChainQueryResponse, Web3Error> {
        Err(Web3Error::Unavailable(
            "optional web3 chain-query adapter is unavailable".into(),
        ))
    }
}

/// Deterministic no-network mock adapter used for policy and trace tests.
#[deprecated(
    note = "Use MockWeb3Provider through Web3SystemServiceProvider for new development/test paths"
)]
#[derive(Debug, Default)]
pub struct MockWeb3Adapter;

#[async_trait]
impl Web3Adapter for MockWeb3Adapter {
    async fn availability(&self) -> Web3Availability {
        Web3Availability::available(vec![
            Web3CapabilityKind::signing(),
            Web3CapabilityKind::transaction(),
            Web3CapabilityKind::chain_query(),
        ])
    }

    async fn sign(
        &self,
        request: &SigningRequest,
        decision: &SigningDecision,
    ) -> Result<SigningProof, Web3Error> {
        if !decision.allowed {
            return Err(Web3Error::DisabledByPolicy(decision.reason.clone()));
        }
        Ok(SigningProof {
            request_id: request.request_id.clone(),
            proof: format!("mock-proof:{}", request.payload_digest),
            algorithm: "mock".into(),
            created_at: Utc::now(),
            metadata: BTreeMap::new(),
        })
    }

    async fn submit_transaction(
        &self,
        request: &TransactionRequest,
        decision: &SigningDecision,
    ) -> Result<TransactionReceipt, Web3Error> {
        if !decision.allowed {
            return Err(Web3Error::DisabledByPolicy(decision.reason.clone()));
        }
        Ok(TransactionReceipt {
            receipt_id: Web3ReceiptId::new(format!("receipt.{}", request.request_id)),
            request_id: request.request_id.clone(),
            transaction_id: Web3TransactionId::new(format!("tx.{}", request.request_id)),
            chain_id: request.chain_id.clone(),
            status: "mock_settled".into(),
            recorded_at: Utc::now(),
            metadata: BTreeMap::from([("mock".into(), "true".into())]),
        })
    }

    async fn query_chain(
        &self,
        request: &ChainQueryRequest,
    ) -> Result<ChainQueryResponse, Web3Error> {
        Ok(ChainQueryResponse {
            request_id: request.request_id.clone(),
            chain_id: request.chain_id.clone(),
            status: "ok".into(),
            result_digest: Some(format!("mock-result:{}", request.method)),
            responded_at: Utc::now(),
            metadata: BTreeMap::new(),
        })
    }
}

/// Runtime-facing Web3 facade.
#[deprecated(
    note = "Use SystemWeb3Client through ServiceRuntime-backed SystemFacade for new Web3 paths"
)]
pub struct Web3Facade {
    adapter: Arc<dyn Web3Adapter>,
    policy: Arc<dyn Web3PolicyEngine>,
    events: Arc<dyn Web3TraceEventSink>,
}

impl Web3Facade {
    /// Create a facade with explicit replaceable dependencies.
    pub fn new(adapter: Arc<dyn Web3Adapter>, policy: Arc<dyn Web3PolicyEngine>) -> Self {
        Self {
            adapter,
            policy,
            events: Arc::new(NoopWeb3TraceEventSink),
        }
    }

    /// Create a facade backed by the Null Object unavailable service.
    pub fn unavailable() -> Self {
        Self::new(
            Arc::new(UnavailableWeb3Adapter),
            Arc::new(DefaultWeb3PolicyEngine::new()),
        )
    }

    /// Attach an event sink for trace/audit integration.
    pub fn with_event_sink(mut self, events: Arc<dyn Web3TraceEventSink>) -> Self {
        self.events = events;
        self
    }

    /// Return optional module availability without executing Web3 behavior.
    pub async fn availability(&self) -> Result<Web3Availability, Web3Error> {
        let availability = self.adapter.availability().await;
        info!(state = %availability.state, "web3 availability checked");
        self.events
            .emit(Web3TraceEvent {
                wallet_id: None,
                chain_id: None,
                capability: "availability".into(),
                operation: "availability_check".into(),
                status: availability.state.clone(),
                request_id: None,
                transaction_id: None,
                receipt_id: None,
                session_id: None,
                task_id: None,
                timestamp: Utc::now(),
                error_code: availability
                    .reason
                    .as_ref()
                    .map(|reason| reason.code.clone()),
                metadata: BTreeMap::new(),
            })
            .await?;
        Ok(availability)
    }

    /// Sign after availability and policy evaluation.
    pub async fn sign(
        &self,
        request: SigningRequest,
        policy: SigningPolicy,
    ) -> Result<SigningProof, Web3Error> {
        let availability = self.availability().await?;
        let decision = self
            .policy
            .evaluate(&request.request_id, &availability, &policy, "sign")?;
        let proof = self.adapter.sign(&request, &decision).await?;
        self.emit_request_event(
            "signing",
            "sign",
            "ok",
            Some(&request.wallet_id.to_string()),
            &request.chain_id,
            &request.request_id,
            request.session_id.clone(),
            request.task_id.clone(),
            None,
        )
        .await?;
        Ok(proof)
    }

    /// Submit a transaction after availability and policy evaluation.
    pub async fn submit_transaction(
        &self,
        request: TransactionRequest,
        policy: SigningPolicy,
    ) -> Result<TransactionReceipt, Web3Error> {
        let availability = self.availability().await?;
        let decision =
            self.policy
                .evaluate(&request.request_id, &availability, &policy, "transaction")?;
        let receipt = self.adapter.submit_transaction(&request, &decision).await?;
        self.emit_request_event(
            "transaction",
            "submit_transaction",
            &receipt.status,
            Some(&request.wallet_id.to_string()),
            &request.chain_id,
            &request.request_id,
            request.session_id.clone(),
            request.task_id.clone(),
            Some(&receipt),
        )
        .await?;
        Ok(receipt)
    }

    /// Query chain state when the optional module is available.
    pub async fn query_chain(
        &self,
        request: ChainQueryRequest,
    ) -> Result<ChainQueryResponse, Web3Error> {
        let availability = self.availability().await?;
        if !availability.is_available() {
            warn!(
                request_id = %request.request_id,
                state = %availability.state,
                "web3 chain query rejected because module is unavailable"
            );
            return Err(Web3Error::Unavailable(
                "optional web3 chain query is unavailable".into(),
            ));
        }
        let response = self.adapter.query_chain(&request).await?;
        self.emit_request_event(
            "chain_query",
            "query_chain",
            &response.status,
            None,
            &request.chain_id,
            &request.request_id,
            request.session_id.clone(),
            request.task_id.clone(),
            None,
        )
        .await?;
        Ok(response)
    }

    async fn emit_request_event(
        &self,
        capability: &str,
        operation: &str,
        status: &str,
        wallet_id: Option<&String>,
        chain_id: &ChainId,
        request_id: &Web3RequestId,
        session_id: Option<String>,
        task_id: Option<String>,
        receipt: Option<&TransactionReceipt>,
    ) -> Result<(), Web3Error> {
        self.events
            .emit(Web3TraceEvent {
                wallet_id: wallet_id.cloned(),
                chain_id: Some(chain_id.to_string()),
                capability: capability.into(),
                operation: operation.into(),
                status: status.into(),
                request_id: Some(request_id.to_string()),
                transaction_id: receipt.map(|value| value.transaction_id.to_string()),
                receipt_id: receipt.map(|value| value.receipt_id.to_string()),
                session_id,
                task_id,
                timestamp: Utc::now(),
                error_code: None,
                metadata: BTreeMap::new(),
            })
            .await
    }
}
