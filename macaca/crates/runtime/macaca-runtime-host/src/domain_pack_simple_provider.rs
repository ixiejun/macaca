//! Generic provider-neutral adapters for packs whose runtime surface is reference based.
//!
//! The macro keeps lifecycle, admission ordering, redaction, quota and unavailable
//! semantics identical while each pack retains a typed service and Strategy name.
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tokio::sync::RwLock;

macro_rules! define_simple_pack {
    ($module:ident, $strategy:ident, $provider:ident, $pack:ident, $service:ident, $commands:ident, $kind:literal) => {
        pub mod $module {
            use super::*;
            use macaca_proto::domain_pack_contract::$module::{$commands, $pack, $service};
            pub trait $strategy: Send + Sync {
                fn validate_command(&self, command: &str) -> ServiceResult<()>;
                fn provider_class(&self) -> &'static str;
            }
            #[derive(Debug, Clone)]
            pub struct ConfiguredStrategy { commands: BTreeSet<String>, provider_class: &'static str }
            impl ConfiguredStrategy {
                pub fn mock() -> Self { Self { commands: $commands.iter().map(|v| (*v).to_string()).collect(), provider_class: "mock" } }
                pub fn with_commands<I, S>(commands: I) -> Self where I: IntoIterator<Item=S>, S: Into<String> { Self { commands: commands.into_iter().map(Into::into).collect(), provider_class: "mock" } }
                pub fn unavailable() -> Self { Self { commands: BTreeSet::new(), provider_class: "unavailable" } }
            }
            impl $strategy for ConfiguredStrategy {
                fn validate_command(&self, command: &str) -> ServiceResult<()> { self.commands.contains(command).then_some(()).ok_or_else(|| ServiceError::UnsupportedCommand(concat!($kind, "_command_unsupported").into())) }
                fn provider_class(&self) -> &'static str { self.provider_class }
            }
            pub struct $provider { descriptor: ServiceDescriptor, references: RwLock<BTreeMap<String,String>>, unavailable_reason: Option<String>, strategy: Arc<dyn $strategy> }
            impl $provider {
                pub fn mock() -> Self { Self::new(None, Arc::new(ConfiguredStrategy::mock())) }
                pub fn mock_with_commands<I,S>(commands: I) -> Self where I: IntoIterator<Item=S>, S: Into<String> { Self::new(None, Arc::new(ConfiguredStrategy::with_commands(commands))) }
                pub fn unavailable(reason: impl Into<String>) -> Self { Self::new(Some(reason.into()), Arc::new(ConfiguredStrategy::unavailable())) }
                fn new(reason: Option<String>, strategy: Arc<dyn $strategy>) -> Self { Self { descriptor: descriptor(), references: RwLock::new(BTreeMap::new()), unavailable_reason: reason, strategy } }
                pub async fn snapshot(&self) -> BTreeMap<String,String> { BTreeMap::from([("pack_id".into(), $pack.into()), ("provider_class".into(), self.strategy.provider_class().into()), ("reference_count".into(), self.references.read().await.len().min(256).to_string()), ("redaction_profile".into(), "opaque_references_metadata_only".into())]) }
                async fn clear(&self) { self.references.write().await.clear(); }
            }
            #[async_trait]
            impl SystemService for $provider {
                fn descriptor(&self) -> ServiceDescriptor { self.descriptor.clone() }
                async fn start(&self) -> ServiceResult<()> { Ok(()) }
                async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
                    let trace = domain_pack_command_trace(&command)?;
                    if let Some(reason) = &self.unavailable_reason { return Err(ServiceError::ServiceUnavailable(sanitize(reason))); }
                    if !$commands.contains(&command.name.as_str()) { return Err(ServiceError::UnsupportedCommand("command_not_declared".into())); }
                    self.strategy.validate_command(command.name.as_str())?;
                    if let Some(reason) = denied(&command.payload) { return Err(ServiceError::DisabledByPolicy(reason.into())); }
                    if self.references.read().await.len() >= 256 { return Err(ServiceError::DisabledByPolicy("quota_exceeded".into())); }
                    let reference = format!(concat!($kind, ":reference:{}"), trace.trace_id);
                    self.references.write().await.insert(trace.trace_id.clone(), reference.clone());
                    Ok(domain_pack_service_result(serde_json::json!({"status":"ok","reference":reference,"provider_class":self.strategy.provider_class(),"content":"redacted","replay_ref":format!("replay:{}", trace.trace_id)}), trace, self.strategy.provider_class()))
                }
                async fn stop(&self) -> ServiceResult<()> { self.clear().await; Ok(()) }
                async fn cleanup(&self) -> ServiceResult<()> { self.clear().await; Ok(()) }
                async fn health(&self) -> ServiceResult<ServiceHealth> { Ok(self.unavailable_reason.as_ref().map_or(ServiceHealth::Healthy, |r| ServiceHealth::Unavailable { reason: sanitize(r) })) }
            }
            pub fn descriptor() -> ServiceDescriptor { let mut d = ServiceDescriptor::new(KernelServiceId::new($service), ServiceType::new(concat!($kind, ".service")), TraceSchemaRef::new(concat!($kind, ".replay.v1"))); d.metadata.insert("pack_id".into(), $pack.into()); d.metadata.insert("command_count".into(), $commands.len().to_string()); d }
            fn denied(payload: &serde_json::Value) -> Option<&'static str> { ["policy_denied","consent_denied","entitlement_denied","approval_required","resource_denied","unsupported","stale_data","timeout","cancelled"].into_iter().find(|key| payload.get(*key).and_then(serde_json::Value::as_bool) == Some(true)) }
            fn sanitize(value: &str) -> String { value.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(64).collect() }
            #[cfg(test)]
            mod tests {
                use super::*;
                use macaca_kernel::SystemService;
                use macaca_proto::{ServiceCommandName, TraceContext};

                fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
                    ServiceCommand::with_trace(
                        ServiceCommandName::new(name),
                        payload,
                        TraceContext::new(concat!($kind, "-provider-test")),
                    )
                }

                #[tokio::test]
                async fn unavailable_is_explicit_and_retains_no_reference() {
                    let provider = $provider::unavailable("provider unavailable");
                    let result = provider.call(command($commands[0], serde_json::json!({}))).await;
                    assert!(matches!(result, Err(ServiceError::ServiceUnavailable(_))));
                    assert_eq!(provider.snapshot().await["reference_count"], "0");
                }

                #[tokio::test]
                async fn policy_denial_precedes_reference_retention() {
                    let provider = $provider::mock();
                    let result = provider
                        .call(command($commands[0], serde_json::json!({"policy_denied": true})))
                        .await;
                    assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
                    assert_eq!(provider.snapshot().await["reference_count"], "0");
                }
            }
        }
    };
}

define_simple_pack!(
    finance_portfolio,
    FinancePortfolioProviderStrategy,
    FinancePortfolioSystemServiceProvider,
    FINANCE_PORTFOLIO_PACK_ID,
    FINANCE_PORTFOLIO_SERVICE_ID,
    FINANCE_PORTFOLIO_COMMANDS,
    "portfolio"
);
define_simple_pack!(
    finance_stock,
    FinanceStockProviderStrategy,
    FinanceStockSystemServiceProvider,
    FINANCE_STOCK_PACK_ID,
    FINANCE_STOCK_SERVICE_ID,
    FINANCE_STOCK_COMMANDS,
    "stock"
);
define_simple_pack!(
    location_geocode,
    LocationGeocodeProviderStrategy,
    LocationGeocodeSystemServiceProvider,
    LOCATION_GEOCODE_PACK_ID,
    LOCATION_GEOCODE_SERVICE_ID,
    LOCATION_GEOCODE_COMMANDS,
    "geocode"
);
define_simple_pack!(
    location_maps,
    LocationMapsProviderStrategy,
    LocationMapsSystemServiceProvider,
    LOCATION_MAPS_PACK_ID,
    LOCATION_MAPS_SERVICE_ID,
    LOCATION_MAPS_COMMANDS,
    "maps"
);
define_simple_pack!(
    location_route,
    LocationRouteProviderStrategy,
    LocationRouteSystemServiceProvider,
    LOCATION_ROUTE_PACK_ID,
    LOCATION_ROUTE_SERVICE_ID,
    LOCATION_ROUTE_COMMANDS,
    "route"
);
define_simple_pack!(
    finance_invoice,
    FinanceInvoiceProviderStrategy,
    FinanceInvoiceSystemServiceProvider,
    FINANCE_INVOICE_PACK_ID,
    FINANCE_INVOICE_SERVICE_ID,
    FINANCE_INVOICE_COMMANDS,
    "invoice"
);
define_simple_pack!(
    media_image,
    MediaImageProviderStrategy,
    MediaImageSystemServiceProvider,
    MEDIA_IMAGE_PACK_ID,
    MEDIA_IMAGE_SERVICE_ID,
    MEDIA_IMAGE_COMMANDS,
    "image"
);
define_simple_pack!(
    media_video,
    MediaVideoProviderStrategy,
    MediaVideoSystemServiceProvider,
    MEDIA_VIDEO_PACK_ID,
    MEDIA_VIDEO_SERVICE_ID,
    MEDIA_VIDEO_COMMANDS,
    "video"
);
define_simple_pack!(
    office_document,
    OfficeDocumentProviderStrategy,
    OfficeDocumentSystemServiceProvider,
    OFFICE_DOCUMENT_PACK_ID,
    OFFICE_DOCUMENT_SERVICE_ID,
    OFFICE_DOCUMENT_COMMANDS,
    "document"
);
define_simple_pack!(
    office_forms,
    OfficeFormsProviderStrategy,
    OfficeFormsSystemServiceProvider,
    OFFICE_FORMS_PACK_ID,
    OFFICE_FORMS_SERVICE_ID,
    OFFICE_FORMS_COMMANDS,
    "forms"
);
define_simple_pack!(
    office_pdf,
    OfficePdfProviderStrategy,
    OfficePdfSystemServiceProvider,
    OFFICE_PDF_PACK_ID,
    OFFICE_PDF_SERVICE_ID,
    OFFICE_PDF_COMMANDS,
    "pdf"
);
define_simple_pack!(
    office_presentation,
    OfficePresentationProviderStrategy,
    OfficePresentationSystemServiceProvider,
    OFFICE_PRESENTATION_PACK_ID,
    OFFICE_PRESENTATION_SERVICE_ID,
    OFFICE_PRESENTATION_COMMANDS,
    "presentation"
);
define_simple_pack!(
    office_spreadsheet,
    OfficeSpreadsheetProviderStrategy,
    OfficeSpreadsheetSystemServiceProvider,
    OFFICE_SPREADSHEET_PACK_ID,
    OFFICE_SPREADSHEET_SERVICE_ID,
    OFFICE_SPREADSHEET_COMMANDS,
    "spreadsheet"
);
define_simple_pack!(
    commerce_entitlement,
    CommerceEntitlementProviderStrategy,
    CommerceEntitlementSystemServiceProvider,
    COMMERCE_ENTITLEMENT_PACK_ID,
    COMMERCE_ENTITLEMENT_SERVICE_ID,
    COMMERCE_ENTITLEMENT_COMMANDS,
    "entitlement"
);
define_simple_pack!(
    commerce_receipt,
    CommerceReceiptProviderStrategy,
    CommerceReceiptSystemServiceProvider,
    COMMERCE_RECEIPT_PACK_ID,
    COMMERCE_RECEIPT_SERVICE_ID,
    COMMERCE_RECEIPT_COMMANDS,
    "receipt"
);
define_simple_pack!(
    media_rendering,
    MediaRenderingProviderStrategy,
    MediaRenderingSystemServiceProvider,
    MEDIA_RENDERING_PACK_ID,
    MEDIA_RENDERING_SERVICE_ID,
    MEDIA_RENDERING_COMMANDS,
    "rendering"
);
define_simple_pack!(
    ai_embedding,
    AiEmbeddingProviderStrategy,
    AiEmbeddingSystemServiceProvider,
    AI_EMBEDDING_PACK_ID,
    AI_EMBEDDING_SERVICE_ID,
    AI_EMBEDDING_COMMANDS,
    "embedding"
);
define_simple_pack!(
    ai_llm,
    AiLlmProviderStrategy,
    AiLlmSystemServiceProvider,
    AI_LLM_PACK_ID,
    AI_LLM_SERVICE_ID,
    AI_LLM_COMMANDS,
    "llm"
);
define_simple_pack!(
    ai_model_evaluation,
    AiModelEvaluationProviderStrategy,
    AiModelEvaluationSystemServiceProvider,
    AI_MODEL_EVALUATION_PACK_ID,
    AI_MODEL_EVALUATION_SERVICE_ID,
    AI_MODEL_EVALUATION_COMMANDS,
    "model_evaluation"
);
define_simple_pack!(
    ai_rerank,
    AiRerankProviderStrategy,
    AiRerankSystemServiceProvider,
    AI_RERANK_PACK_ID,
    AI_RERANK_SERVICE_ID,
    AI_RERANK_COMMANDS,
    "rerank"
);
define_simple_pack!(
    ai_speech,
    AiSpeechProviderStrategy,
    AiSpeechSystemServiceProvider,
    AI_SPEECH_PACK_ID,
    AI_SPEECH_SERVICE_ID,
    AI_SPEECH_COMMANDS,
    "speech"
);
define_simple_pack!(
    ai_vision,
    AiVisionProviderStrategy,
    AiVisionSystemServiceProvider,
    AI_VISION_PACK_ID,
    AI_VISION_SERVICE_ID,
    AI_VISION_COMMANDS,
    "vision"
);
define_simple_pack!(
    workflow_delegation,
    WorkflowDelegationProviderStrategy,
    WorkflowDelegationSystemServiceProvider,
    WORKFLOW_DELEGATION_PACK_ID,
    WORKFLOW_DELEGATION_SERVICE_ID,
    WORKFLOW_DELEGATION_COMMANDS,
    "delegation"
);
define_simple_pack!(
    workflow_recovery,
    WorkflowRecoveryProviderStrategy,
    WorkflowRecoverySystemServiceProvider,
    WORKFLOW_RECOVERY_PACK_ID,
    WORKFLOW_RECOVERY_SERVICE_ID,
    WORKFLOW_RECOVERY_COMMANDS,
    "recovery"
);
define_simple_pack!(
    workflow_schedule,
    WorkflowScheduleProviderStrategy,
    WorkflowScheduleSystemServiceProvider,
    WORKFLOW_SCHEDULE_PACK_ID,
    WORKFLOW_SCHEDULE_SERVICE_ID,
    WORKFLOW_SCHEDULE_COMMANDS,
    "schedule"
);
