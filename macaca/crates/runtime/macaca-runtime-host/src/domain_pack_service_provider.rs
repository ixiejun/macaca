//! Built-in domain-pack service adapters for contract-driven applications.
//!
//! Macaca applications must depend on generic OS services instead of forcing
//! the runtime host to know application-specific workflows.  This module keeps
//! that boundary explicit: it exposes provider-neutral `SystemService`
//! implementations for services declared by domain packs, and the Web/CLI host
//! can register those services once during startup.  Applications then request
//! them through manifest service contracts and WASM `service.call` imports.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_llm::LlmProvider;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, LlmMessage, LlmOptions, MacacaError, MacacaResult,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceType, TraceContext, TraceSchemaRef,
};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Stable service id exposed by the finance domain pack for quote-like data.
pub const FINANCE_MARKET_DATA_SERVICE_ID: &str = "service.market_data";
/// Stable service id exposed by the finance domain pack for fundamentals.
pub const FINANCE_FINANCIALS_SERVICE_ID: &str = "service.financials";
/// Stable service id exposed by the finance domain pack for summarized news.
pub const FINANCE_NEWS_DIGEST_SERVICE_ID: &str = "service.news_digest";
/// Stable service id exposed by the finance domain pack for model analysis.
pub const FINANCE_LLM_ANALYSIS_SERVICE_ID: &str = "service.llm.analysis";

/// Command accepted by deterministic finance information services.
pub const FINANCE_LOOKUP_COMMAND: &str = "finance.lookup";
/// Command accepted by the LLM-backed finance analysis service.
pub const FINANCE_ANALYZE_COMMAND: &str = "finance.analyze";

/// Common result builder used by all domain-pack service adapters.
///
/// Keeping result construction in one helper makes metadata and cleanup
/// semantics consistent across services.  The metadata is intentionally
/// generic and audit-friendly; it never includes application names or secrets.
fn service_result(
    output: Value,
    trace: TraceContext,
    provider_class: &'static str,
) -> ServiceCallResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("provider_class".into(), provider_class.into());
    metadata.insert("domain_pack".into(), "pack.finance.v1".into());
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata,
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Extract a trace context or fail before provider logic runs.
fn command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

/// Best-effort symbol parser shared by finance pack services.
///
/// The guest SDK remains responsible for strict validation.  Host-side domain
/// services still normalize inputs defensively because malformed requests can
/// arrive from old applications, direct SDK callers, or future IPC bridges.
fn extract_symbol(payload: &Value) -> String {
    let candidate = payload
        .get("ticker")
        .or_else(|| payload.get("symbol"))
        .or_else(|| payload.get("input"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    candidate
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-')
        .find(|part| part.chars().any(|ch| ch.is_ascii_alphabetic()))
        .unwrap_or(candidate)
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-')
        .to_ascii_uppercase()
}

/// Build a generic descriptor for one domain-pack service.
fn finance_descriptor(service_id: &str, service_kind: &str) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new(service_kind),
        TraceSchemaRef::new("trace.domain_pack.finance.v1"),
    );
    descriptor
        .metadata
        .insert("domain_pack".into(), "pack.finance.v1".into());
    descriptor
        .metadata
        .insert("contract_version".into(), "v1".into());
    descriptor
}

/// Deterministic finance data provider used for market, financial, and news services.
///
/// This adapter deliberately does not know about any application.  It is a
/// generic domain-pack provider that emits structured fixture-grade data for
/// contract testing and local development.  Real deployments can replace the
/// same service ids with remote data providers without changing WASM apps.
pub struct FinanceDataSystemServiceProvider {
    descriptor: ServiceDescriptor,
    service_kind: &'static str,
}

impl FinanceDataSystemServiceProvider {
    /// Create a market-data provider for `pack.finance.v1`.
    pub fn market_data() -> Self {
        Self::new(FINANCE_MARKET_DATA_SERVICE_ID, "finance.market_data")
    }

    /// Create a financials provider for `pack.finance.v1`.
    pub fn financials() -> Self {
        Self::new(FINANCE_FINANCIALS_SERVICE_ID, "finance.financials")
    }

    /// Create a news-digest provider for `pack.finance.v1`.
    pub fn news_digest() -> Self {
        Self::new(FINANCE_NEWS_DIGEST_SERVICE_ID, "finance.news_digest")
    }

    fn new(service_id: &'static str, service_kind: &'static str) -> Self {
        Self {
            descriptor: finance_descriptor(service_id, service_kind),
            service_kind,
        }
    }

    fn output_for(&self, symbol: &str, payload: &Value) -> Value {
        match self.service_kind {
            "finance.market_data" => json!({
                "symbol": symbol,
                "currency": "USD",
                "price": 197.23,
                "day_change_percent": 0.84,
                "as_of": "fixture.realtime",
                "source": "domain_pack.finance.v1.local_fixture",
                "input": payload,
            }),
            "finance.financials" => json!({
                "symbol": symbol,
                "revenue_growth_yoy_percent": 6.1,
                "gross_margin_percent": 46.6,
                "free_cash_flow_quality": "strong",
                "debt_risk": "low",
                "source": "domain_pack.finance.v1.local_fixture",
                "input": payload,
            }),
            _ => json!({
                "symbol": symbol,
                "sentiment": "mixed_positive",
                "items": [
                    "Product demand remains a monitored driver.",
                    "Macro rates and valuation sensitivity remain key risks.",
                    "Recent coverage emphasizes durable services revenue."
                ],
                "source": "domain_pack.finance.v1.local_fixture",
                "input": payload,
            }),
        }
    }
}

#[async_trait]
impl SystemService for FinanceDataSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            service_kind = self.service_kind,
            "finance domain-pack data service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        let symbol = extract_symbol(&command.payload);
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            symbol = %symbol,
            "finance domain-pack data service accepted command"
        );
        if command.name.as_str() != FINANCE_LOOKUP_COMMAND {
            warn!(
                service_id = %self.descriptor.id,
                command = %command.name,
                trace_id = %trace.trace_id,
                "finance domain-pack data service rejected unsupported command"
            );
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        Ok(service_result(
            self.output_for(&symbol, &command.payload),
            trace,
            "finance_domain_pack_data",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            service_kind = self.service_kind,
            "finance domain-pack data service stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            service_kind = self.service_kind,
            "finance domain-pack data service cleaned up"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// LLM-backed analysis adapter for the finance domain pack.
///
/// This provider is intentionally a Bridge: WASM applications call the generic
/// `service.llm.analysis` contract, while the adapter delegates model execution
/// to the host's configured `LlmProvider`.  The application never receives API
/// keys and Macaca never learns application-specific workflow names.
pub struct FinanceLlmAnalysisSystemServiceProvider {
    descriptor: ServiceDescriptor,
    llm: Arc<dyn LlmProvider>,
}

impl FinanceLlmAnalysisSystemServiceProvider {
    /// Create an analysis provider over the host-selected LLM strategy.
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self {
            descriptor: finance_descriptor(FINANCE_LLM_ANALYSIS_SERVICE_ID, "finance.llm_analysis"),
            llm,
        }
    }

    fn prompt(symbol: &str, payload: &Value) -> Vec<LlmMessage> {
        vec![
            LlmMessage::system(
                "You are a finance analysis service inside an application runtime. \
                 Produce concise, non-advisory analysis from the provided structured payload. \
                 Do not claim to have live market access beyond the supplied data.",
            ),
            LlmMessage::user(format!(
                "Analyze symbol {symbol}. Use this JSON payload as the complete evidence set: {payload}",
            )),
        ]
    }
}

#[async_trait]
impl SystemService for FinanceLlmAnalysisSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            llm_provider = self.llm.name(),
            "finance domain-pack llm analysis service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        let symbol = extract_symbol(&command.payload);
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            symbol = %symbol,
            "finance domain-pack llm analysis accepted command"
        );
        if command.name.as_str() != FINANCE_ANALYZE_COMMAND {
            warn!(
                service_id = %self.descriptor.id,
                command = %command.name,
                trace_id = %trace.trace_id,
                "finance domain-pack llm analysis rejected unsupported command"
            );
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let options = LlmOptions {
            max_tokens: Some(900),
            temperature: Some(0.2),
            ..LlmOptions::default()
        };
        let response = self
            .llm
            .chat(Self::prompt(&symbol, &command.payload), &options)
            .await
            .map_err(service_adapter_error)?;
        Ok(service_result(
            json!({
                "symbol": symbol,
                "analysis": response.content,
                "model": response.model,
                "finish_reason": response.finish_reason,
                "usage": response.usage,
            }),
            trace,
            "finance_domain_pack_llm",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            llm_provider = self.llm.name(),
            "finance domain-pack llm analysis service stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            llm_provider = self.llm.name(),
            "finance domain-pack llm analysis service cleaned up"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Started service ids returned by the domain-pack bootstrap boundary.
#[derive(Clone, Debug, Default)]
pub struct DomainPackRuntimeBundle {
    pub started_services: Vec<KernelServiceId>,
}

/// Register and start built-in domain-pack services used by contract apps.
///
/// The function is deliberately generic at the app boundary.  It registers
/// services by domain-pack service id, so any application declaring
/// `pack.finance.v1` can use them without runtime code mentioning that app.
pub async fn bootstrap_builtin_domain_pack_services(
    runtime: Arc<ServiceRuntime>,
    llm: Arc<dyn LlmProvider>,
    trace_prefix: impl Into<String>,
) -> MacacaResult<DomainPackRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    let services: Vec<(ServiceDescriptor, Arc<dyn SystemService>, &'static str)> = vec![
        (
            finance_descriptor(FINANCE_MARKET_DATA_SERVICE_ID, "finance.market_data"),
            Arc::new(FinanceDataSystemServiceProvider::market_data()),
            "finance-market-data",
        ),
        (
            finance_descriptor(FINANCE_FINANCIALS_SERVICE_ID, "finance.financials"),
            Arc::new(FinanceDataSystemServiceProvider::financials()),
            "finance-financials",
        ),
        (
            finance_descriptor(FINANCE_NEWS_DIGEST_SERVICE_ID, "finance.news_digest"),
            Arc::new(FinanceDataSystemServiceProvider::news_digest()),
            "finance-news-digest",
        ),
        (
            finance_descriptor(FINANCE_LLM_ANALYSIS_SERVICE_ID, "finance.llm_analysis"),
            Arc::new(FinanceLlmAnalysisSystemServiceProvider::new(llm)),
            "finance-llm-analysis",
        ),
    ];
    let mut bundle = DomainPackRuntimeBundle::default();
    for (descriptor, service, trace_suffix) in services {
        let service_id = descriptor.id.clone();
        let trace = TraceContext::new(format!("{trace_prefix}-{trace_suffix}"));
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "domain-pack service registering provider"
        );
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    descriptor, service,
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(runtime_error)?;
        runtime
            .start(&service_id, trace.clone())
            .await
            .map_err(runtime_error)?;
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "domain-pack service started"
        );
        bundle.started_services.push(service_id);
    }
    Ok(bundle)
}

fn service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}
