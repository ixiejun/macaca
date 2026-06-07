//! LLM-backed analysis service for the finance domain pack.
//!
//! This provider is a Bridge: WASM applications call the generic
//! `service.llm.analysis` contract, while the adapter delegates model execution
//! to the host's configured `LlmProvider`.  The application never receives API
//! keys and the OS never learns application-specific workflow names.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_llm::LlmProvider;
use macaca_proto::{
    LlmMessage, LlmOptions, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult,
};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::contract::{
    extract_symbol, finance_descriptor, finance_service_result, FINANCE_ANALYZE_COMMAND,
    FINANCE_LLM_ANALYSIS_SERVICE_ID,
};

/// LLM-backed analysis adapter for the finance domain pack.
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

    /// Build the provider prompt from already fetched finance evidence.
    ///
    /// The model is constrained to the payload so it does not invent hidden
    /// market access.  The host fetches data first; the LLM explains only that
    /// evidence in the returned analysis block.
    fn prompt(symbol: &str, payload: &Value) -> Vec<LlmMessage> {
        let signal_mode = payload
            .get("analysis_type")
            .and_then(Value::as_str)
            .map(|value| value.eq_ignore_ascii_case("signal"))
            .unwrap_or(false);
        let output_contract = if signal_mode {
            "Because analysis_type is signal, start with a compact signal block: \
             Signal: BUY, SELL, or HOLD; Confidence: low, medium, or high; \
             Buy/entry zone; Sell/take-profit zone; Stop/invalidation; \
             Evidence. Use only the supplied market data, news, and delegated \
             agent outputs. Keep it non-advisory and make uncertainty explicit."
        } else {
            "Summarize trend, risk, support/resistance context, and possible \
             watch zones without issuing personalized financial advice."
        };
        vec![
            LlmMessage::system(format!(
                "You are a finance analysis service inside an application runtime. \
                 Produce concise, non-advisory analysis from the provided structured payload. \
                 Do not claim to have live market access beyond the supplied data. \
                 {output_contract}",
            )),
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
        let trace =
            macaca_runtime_host::domain_pack_service_provider::command_trace(&command)?;
        let symbol = extract_symbol(&command.payload)?;
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
            max_tokens: Some(1800),
            temperature: Some(0.2),
            ..LlmOptions::default()
        };
        let response = self
            .llm
            .chat(Self::prompt(&symbol, &command.payload), &options)
            .await
            .map_err(macaca_runtime_host::domain_pack_service_provider::service_adapter_error)?;
        Ok(finance_service_result(
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
