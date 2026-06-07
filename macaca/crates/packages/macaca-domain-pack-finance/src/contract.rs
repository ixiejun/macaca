//! Finance domain-pack contract constants and shared validation helpers.
//!
//! This module owns the stable service identifiers and descriptor metadata that
//! WASM applications declare through manifest `use_packs`.  Keeping the contract
//! in the optional package crate prevents the base runtime host from embedding
//! finance-specific strings while still giving composition roots a single import
//! surface for registration and catalog metadata.

use std::collections::BTreeMap;

use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceDescriptor, ServiceError,
    ServiceResult, ServiceType, TraceContext, TraceSchemaRef,
};
use serde_json::Value;

/// Stable pack identifier referenced by application manifests.
pub const FINANCE_PACK_ID: &str = "pack.finance.v1";

/// Stable service id used by the finance pack for quote-like data.
pub const FINANCE_MARKET_DATA_SERVICE_ID: &str = "service.market_data";
/// Stable service id used by the finance pack for fundamentals.
pub const FINANCE_FINANCIALS_SERVICE_ID: &str = "service.financials";
/// Stable service id used by the finance pack for summarized news.
pub const FINANCE_NEWS_DIGEST_SERVICE_ID: &str = "service.news_digest";
/// Stable service id used by the finance pack for model analysis.
pub const FINANCE_LLM_ANALYSIS_SERVICE_ID: &str = "service.llm.analysis";

/// Command accepted by deterministic finance information services.
pub const FINANCE_LOOKUP_COMMAND: &str = "finance.lookup";
/// Command accepted by the LLM-backed finance analysis service.
pub const FINANCE_ANALYZE_COMMAND: &str = "finance.analyze";

/// Build a descriptor for one finance-pack service.
///
/// Descriptors carry pack metadata for audit replay without requiring the
/// runtime host to understand finance semantics.
pub fn finance_descriptor(service_id: &str, service_kind: &str) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new(service_kind),
        TraceSchemaRef::new("trace.domain_pack.finance.v1"),
    );
    descriptor
        .metadata
        .insert("domain_pack".into(), FINANCE_PACK_ID.into());
    descriptor
        .metadata
        .insert("contract_version".into(), "v1".into());
    descriptor
}

/// Extract a typed finance symbol from a service payload.
///
/// Applications must normalize user language into typed `symbol`/`ticker`
/// fields before crossing the OS boundary.  This helper rejects prompt-shaped
/// inputs so audit traces remain honest about typed service calls.
pub fn extract_symbol(payload: &Value) -> ServiceResult<String> {
    let Some((field, raw_symbol)) = ["ticker", "symbol"].into_iter().find_map(|field| {
        payload
            .get(field)
            .and_then(Value::as_str)
            .map(|value| (field, value))
    }) else {
        return Err(ServiceError::InvalidArgument(
            "finance pack requires typed `symbol` or `ticker` argument".into(),
        ));
    };
    let symbol = raw_symbol.trim().to_ascii_uppercase();
    if symbol.is_empty()
        || !(1..=12).contains(&symbol.len())
        || !symbol.chars().all(|ch| ch.is_ascii_alphanumeric())
    {
        return Err(ServiceError::InvalidArgument(format!(
            "finance pack `{field}` must be a typed alphanumeric symbol, not a prompt"
        )));
    }
    Ok(symbol)
}

/// Return whether the caller is requesting crypto-shaped finance data.
///
/// Crypto remains part of the finance domain pack; this helper only selects a
/// live market-data adapter for generic `asset_class: crypto` requests.
pub fn is_crypto_asset(payload: &Value) -> bool {
    payload
        .get("asset_class")
        .and_then(Value::as_str)
        .map(|value| value.eq_ignore_ascii_case("crypto"))
        .unwrap_or(false)
}

/// Build a finance-pack service result with pack-scoped audit metadata.
pub fn finance_service_result(
    output: Value,
    trace: TraceContext,
    provider_class: &'static str,
) -> ServiceCallResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("provider_class".into(), provider_class.into());
    metadata.insert("domain_pack".into(), FINANCE_PACK_ID.into());
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata,
        cleanup_hint: Some(CleanupPolicy::None),
    }
}
