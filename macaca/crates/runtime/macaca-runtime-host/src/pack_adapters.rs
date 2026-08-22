//! Public module registry for optional domain-pack adapters.
//!
//! Keeping these declarations behind one facade prevents the runtime-host root
//! from becoming a source-size hotspot while preserving stable module paths.
#[path = "ai_embedding_service_provider.rs"]
pub mod ai_embedding_service_provider;
#[path = "ai_embedding_strategy.rs"]
pub mod ai_embedding_strategy;
#[path = "ai_llm_service_provider.rs"]
pub mod ai_llm_service_provider;
#[path = "ai_llm_strategy.rs"]
pub mod ai_llm_strategy;
#[path = "ai_model_evaluation_service_provider.rs"]
pub mod ai_model_evaluation_service_provider;
#[path = "ai_model_evaluation_strategy.rs"]
pub mod ai_model_evaluation_strategy;
#[path = "ai_rerank_service_provider.rs"]
pub mod ai_rerank_service_provider;
#[path = "ai_rerank_strategy.rs"]
pub mod ai_rerank_strategy;
#[path = "ai_speech_service_provider.rs"]
pub mod ai_speech_service_provider;
#[path = "ai_speech_strategy.rs"]
pub mod ai_speech_strategy;
#[path = "ai_vision_service_provider.rs"]
pub mod ai_vision_service_provider;
#[path = "ai_vision_strategy.rs"]
pub mod ai_vision_strategy;
#[path = "commerce_entitlement_service_provider.rs"]
pub mod commerce_entitlement_service_provider;
#[path = "commerce_entitlement_strategy.rs"]
pub mod commerce_entitlement_strategy;
#[path = "commerce_receipt_service_provider.rs"]
pub mod commerce_receipt_service_provider;
#[path = "commerce_receipt_strategy.rs"]
pub mod commerce_receipt_strategy;
#[path = "finance_invoice_service_provider.rs"]
pub mod finance_invoice_service_provider;
#[path = "finance_invoice_strategy.rs"]
pub mod finance_invoice_strategy;
#[path = "finance_portfolio_service_provider.rs"]
pub mod finance_portfolio_service_provider;
#[path = "finance_portfolio_strategy.rs"]
pub mod finance_portfolio_strategy;
#[path = "finance_stock_service_provider.rs"]
pub mod finance_stock_service_provider;
#[path = "finance_stock_strategy.rs"]
pub mod finance_stock_strategy;
#[path = "location_geocode_service_provider.rs"]
pub mod location_geocode_service_provider;
#[path = "location_geocode_strategy.rs"]
pub mod location_geocode_strategy;
#[path = "location_maps_service_provider.rs"]
pub mod location_maps_service_provider;
#[path = "location_maps_strategy.rs"]
pub mod location_maps_strategy;
#[path = "location_route_service_provider.rs"]
pub mod location_route_service_provider;
#[path = "location_route_strategy.rs"]
pub mod location_route_strategy;
#[path = "media_image_service_provider.rs"]
pub mod media_image_service_provider;
#[path = "media_image_strategy.rs"]
pub mod media_image_strategy;
#[path = "media_rendering_service_provider.rs"]
pub mod media_rendering_service_provider;
#[path = "media_rendering_strategy.rs"]
pub mod media_rendering_strategy;
#[path = "media_video_service_provider.rs"]
pub mod media_video_service_provider;
#[path = "media_video_strategy.rs"]
pub mod media_video_strategy;
#[path = "office_document_service_provider.rs"]
pub mod office_document_service_provider;
#[path = "office_document_strategy.rs"]
pub mod office_document_strategy;
#[path = "office_forms_service_provider.rs"]
pub mod office_forms_service_provider;
#[path = "office_forms_strategy.rs"]
pub mod office_forms_strategy;
#[path = "office_pdf_service_provider.rs"]
pub mod office_pdf_service_provider;
#[path = "office_pdf_strategy.rs"]
pub mod office_pdf_strategy;
#[path = "office_presentation_service_provider.rs"]
pub mod office_presentation_service_provider;
#[path = "office_presentation_strategy.rs"]
pub mod office_presentation_strategy;
#[path = "office_spreadsheet_service_provider.rs"]
pub mod office_spreadsheet_service_provider;
#[path = "office_spreadsheet_strategy.rs"]
pub mod office_spreadsheet_strategy;
#[path = "workflow_delegation_service_provider.rs"]
pub mod workflow_delegation_service_provider;
#[path = "workflow_delegation_service_provider.rs"]
pub mod workflow_delegation_service_provider_alias;
#[path = "workflow_delegation_strategy.rs"]
pub mod workflow_delegation_strategy;
#[path = "workflow_recovery_service_provider.rs"]
pub mod workflow_recovery_service_provider;
#[path = "workflow_recovery_service_provider.rs"]
pub mod workflow_recovery_service_provider_alias;
#[path = "workflow_recovery_strategy.rs"]
pub mod workflow_recovery_strategy;
#[path = "workflow_schedule_service_provider.rs"]
pub mod workflow_schedule_service_provider;
#[path = "workflow_schedule_service_provider.rs"]
pub mod workflow_schedule_service_provider_alias;
#[path = "workflow_schedule_strategy.rs"]
pub mod workflow_schedule_strategy;

pub use crate::domain_pack_simple_provider::{
    ai_embedding::AiEmbeddingSystemServiceProvider, ai_llm::AiLlmSystemServiceProvider,
    ai_model_evaluation::AiModelEvaluationSystemServiceProvider,
    ai_rerank::AiRerankSystemServiceProvider, ai_speech::AiSpeechSystemServiceProvider,
    ai_vision::AiVisionSystemServiceProvider,
    commerce_entitlement::CommerceEntitlementSystemServiceProvider,
    commerce_receipt::CommerceReceiptSystemServiceProvider,
    finance_invoice::FinanceInvoiceSystemServiceProvider,
    finance_portfolio::FinancePortfolioSystemServiceProvider,
    finance_stock::FinanceStockSystemServiceProvider,
    location_geocode::LocationGeocodeSystemServiceProvider,
    location_maps::LocationMapsSystemServiceProvider,
    location_route::LocationRouteSystemServiceProvider,
    media_image::MediaImageSystemServiceProvider,
    media_rendering::MediaRenderingSystemServiceProvider,
    media_video::MediaVideoSystemServiceProvider,
    office_document::OfficeDocumentSystemServiceProvider,
    office_forms::OfficeFormsSystemServiceProvider, office_pdf::OfficePdfSystemServiceProvider,
    office_presentation::OfficePresentationSystemServiceProvider,
    office_spreadsheet::OfficeSpreadsheetSystemServiceProvider,
    workflow_delegation::WorkflowDelegationSystemServiceProvider,
    workflow_recovery::WorkflowRecoverySystemServiceProvider,
    workflow_schedule::WorkflowScheduleSystemServiceProvider,
};
