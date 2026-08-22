//! Runtime-host market-data provider returning bounded, attributed references only.
use crate::finance_market_data_strategy::{
    ConfiguredFinanceMarketDataStrategy, FinanceMarketDataProviderStrategy,
};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::finance_market_data::{
    FINANCE_MARKET_DATA_COMMANDS, FINANCE_MARKET_DATA_PACK_ID, FINANCE_MARKET_DATA_SERVICE_ID,
    FINANCE_MARKET_DATA_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;
/// Stores request references and bounded metadata, never licensed payloads or account data.
pub struct FinanceMarketDataSystemServiceProvider {
    descriptor: ServiceDescriptor,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn FinanceMarketDataProviderStrategy>,
}
impl FinanceMarketDataSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None, Arc::new(ConfiguredFinanceMarketDataStrategy::mock()))
    }
    pub fn mock_with_commands<I, S>(c: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            None,
            Arc::new(ConfiguredFinanceMarketDataStrategy::with_commands(c)),
        )
    }
    pub fn unavailable(r: impl Into<String>) -> Self {
        Self::new(
            Some(r.into()),
            Arc::new(ConfiguredFinanceMarketDataStrategy::unavailable()),
        )
    }
    fn new(r: Option<String>, s: Arc<dyn FinanceMarketDataProviderStrategy>) -> Self {
        Self {
            descriptor: finance_market_data_service_descriptor(),
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason: r,
            strategy: s,
        }
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("pack_id".into(), FINANCE_MARKET_DATA_PACK_ID.into()),
            (
                "provider_class".into(),
                self.strategy.provider_class().into(),
            ),
            (
                "reference_count".into(),
                self.references.read().await.len().min(256).to_string(),
            ),
            (
                "redaction_profile".into(),
                "instrument_quote_trade_bar_corporate_action_freshness_refs_only".into(),
            ),
        ])
    }
    async fn shutdown(&self) {
        self.references.write().await.clear();
    }
}
#[async_trait]
impl SystemService for FinanceMarketDataSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn call(&self, c: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let t = domain_pack_command_trace(&c)?;
        if let Some(r) = &self.unavailable_reason {
            return Err(ServiceError::ServiceUnavailable(sanitize(r)));
        }
        if !FINANCE_MARKET_DATA_COMMANDS.contains(&c.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "market_data_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(c.name.as_str())?;
        if let Some(r) = denied(&c.payload) {
            return Err(ServiceError::DisabledByPolicy(r.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("market-data:reference:{}", t.trace_id);
        self.references
            .write()
            .await
            .insert(t.trace_id.clone(), reference.clone());
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","market_data_ref":reference,"provider_class":self.strategy.provider_class(),"freshness":"reported","attribution":"required","content":"redacted","replay_ref":format!("replay:{}",t.trace_id)}),
            t,
            self.strategy.provider_class(),
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.shutdown().await;
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.shutdown().await;
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        self.unavailable_reason
            .as_ref()
            .map_or(Ok(ServiceHealth::Healthy), |r| {
                Ok(ServiceHealth::Unavailable {
                    reason: sanitize(r),
                })
            })
    }
}
fn denied(p: &serde_json::Value) -> Option<&'static str> {
    [
        "policy_denied",
        "symbol_ambiguous",
        "symbol_not_found",
        "exchange_unsupported",
        "asset_class_unsupported",
        "range_too_large",
        "interval_unsupported",
        "adjustment_unsupported",
        "license_denied",
        "attribution_required",
        "stale_data",
        "network_denied",
        "timeout",
        "cancelled",
        "quota_exceeded",
    ]
    .into_iter()
    .find(|k| p.get(*k).and_then(serde_json::Value::as_bool) == Some(true))
}
fn sanitize(v: &str) -> String {
    v.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}
pub fn finance_market_data_service_descriptor() -> ServiceDescriptor {
    let mut d = ServiceDescriptor::new(
        KernelServiceId::new(FINANCE_MARKET_DATA_SERVICE_ID),
        ServiceType::new("finance.market_data"),
        TraceSchemaRef::new("finance.market_data.replay.v1"),
    );
    d.metadata
        .insert("pack_id".into(), FINANCE_MARKET_DATA_PACK_ID.into());
    d.metadata.insert(
        "command_count".into(),
        FINANCE_MARKET_DATA_COMMANDS.len().to_string(),
    );
    d.metadata.insert(
        "trace_event_count".into(),
        FINANCE_MARKET_DATA_TRACE_EVENTS.len().to_string(),
    );
    d
}
