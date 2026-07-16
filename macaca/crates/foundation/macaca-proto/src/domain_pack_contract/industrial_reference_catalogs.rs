use std::collections::{BTreeMap, BTreeSet};

use super::ai_embedding::ai_embedding_pack_definition;
use super::ai_llm::ai_llm_pack_definition;
use super::ai_model_evaluation::ai_model_evaluation_pack_definition;
use super::ai_rerank::ai_rerank_pack_definition;
use super::ai_speech::ai_speech_pack_definition;
use super::ai_vision::ai_vision_pack_definition;
use super::commerce_cart::commerce_cart_pack_definition;
use super::commerce_catalog::commerce_catalog_pack_definition;
use super::commerce_entitlement::commerce_entitlement_pack_definition;
use super::commerce_order::commerce_order_pack_definition;
use super::commerce_payment_intent::commerce_payment_intent_pack_definition;
use super::commerce_receipt::commerce_receipt_pack_definition;
use super::communication_calendar::communication_calendar_pack_definition;
use super::communication_email::communication_email_pack_definition;
use super::communication_inbox::communication_inbox_pack_definition;
use super::communication_messaging::communication_messaging_pack_definition;
use super::communication_notification::communication_notification_pack_definition;
use super::developer_browser_automation::developer_browser_automation_pack_definition;
use super::developer_ci::developer_ci_pack_definition;
use super::developer_code::developer_code_pack_definition;
use super::developer_design_tools::developer_design_tools_pack_definition;
use super::developer_issue_tracker::developer_issue_tracker_pack_definition;
use super::developer_repository::developer_repository_pack_definition;
use super::developer_terminal::developer_terminal_pack_definition;
use super::device_camera::device_camera_pack_definition;
use super::device_foreground_background_host::device_foreground_background_host_pack_definition;
use super::device_local_files::device_local_files_pack_definition;
use super::device_notifications::device_notifications_pack_definition;
use super::device_sensors::device_sensors_pack_definition;
use super::finance_accounting::finance_accounting_pack_definition;
use super::finance_crypto::finance_crypto_pack_definition;
use super::finance_invoice::finance_invoice_pack_definition;
use super::finance_market_data::finance_market_data_pack_definition;
use super::finance_portfolio::finance_portfolio_pack_definition;
use super::finance_stock::finance_stock_pack_definition;
use super::foundation_config::foundation_config_pack_definition;
use super::foundation_filesystem::foundation_filesystem_pack_definition;
use super::foundation_key_value_state::foundation_key_value_state_pack_definition;
use super::foundation_random::foundation_random_pack_definition;
use super::foundation_secrets_reference::foundation_secrets_reference_pack_definition;
use super::foundation_session_state::foundation_session_state_pack_definition;
use super::foundation_time::foundation_time_pack_definition;
use super::identity_account::identity_account_pack_definition;
use super::identity_auth_handoff::identity_auth_handoff_pack_definition;
use super::identity_organization::identity_organization_pack_definition;
use super::identity_profile::identity_profile_pack_definition;
use super::identity_tenant::identity_tenant_pack_definition;
use super::industrial_pack_taxonomy::{IndustrialSubPackEntry, INDUSTRIAL_SUB_PACKS};
use super::knowledge_citations::knowledge_citations_pack_definition;
use super::knowledge_document_parsing::knowledge_document_parsing_pack_definition;
use super::knowledge_graph::knowledge_graph_pack_definition;
use super::knowledge_retrieval::knowledge_retrieval_pack_definition;
use super::knowledge_search::knowledge_search_pack_definition;
use super::knowledge_summarization::knowledge_summarization_pack_definition;
use super::location_geocode::location_geocode_pack_definition;
use super::location_maps::location_maps_pack_definition;
use super::location_place_search::location_place_search_pack_definition;
use super::location_route::location_route_pack_definition;
use super::location_timezone::location_timezone_pack_definition;
use super::media_audio::media_audio_pack_definition;
use super::media_image::media_image_pack_definition;
use super::media_rendering::media_rendering_pack_definition;
use super::media_transcription::media_transcription_pack_definition;
use super::media_video::media_video_pack_definition;
use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackSdkMetadata, DomainPackStability,
};
use super::office_document::office_document_pack_definition;
use super::office_forms::office_forms_pack_definition;
use super::office_pdf::office_pdf_pack_definition;
use super::office_presentation::office_presentation_pack_definition;
use super::office_spreadsheet::office_spreadsheet_pack_definition;
use super::workflow_approval::workflow_approval_pack_definition;
use super::workflow_delegation::workflow_delegation_pack_definition;
use super::workflow_recovery::workflow_recovery_pack_definition;
use super::workflow_review::workflow_review_pack_definition;
use super::workflow_schedule::workflow_schedule_pack_definition;
use super::workflow_task::workflow_task_pack_definition;

/// Return the initial industrial sub-pack taxonomy as preview-unavailable descriptors.
///
/// These entries are intentionally descriptor-only.  They make the industrial catalog visible to
/// SDKs and admission tooling while preserving the serviceization boundary: a sub-pack becomes
/// callable only when an optional package or plugin supplies concrete service mappings through the
/// same catalog composition hook.
pub fn industrial_reference_domain_pack_definitions() -> Vec<DomainPackDefinition> {
    INDUSTRIAL_SUB_PACKS
        .iter()
        .map(|entry| {
            specialized_sub_pack_definition(entry).unwrap_or_else(|| {
                unavailable_sub_pack_definition(entry.family, entry.slug, entry.label)
            })
        })
        .collect()
}

fn specialized_sub_pack_definition(entry: &IndustrialSubPackEntry) -> Option<DomainPackDefinition> {
    match (entry.family, entry.slug) {
        ("foundation", "config") => Some(foundation_config_pack_definition()),
        ("foundation", "filesystem") => Some(foundation_filesystem_pack_definition()),
        ("foundation", "key-value-state") => Some(foundation_key_value_state_pack_definition()),
        ("foundation", "random") => Some(foundation_random_pack_definition()),
        ("foundation", "secrets-reference") => Some(foundation_secrets_reference_pack_definition()),
        ("foundation", "session-state") => Some(foundation_session_state_pack_definition()),
        ("foundation", "time") => Some(foundation_time_pack_definition()),
        ("communication", "email") => Some(communication_email_pack_definition()),
        ("communication", "messaging") => Some(communication_messaging_pack_definition()),
        ("communication", "notification") => Some(communication_notification_pack_definition()),
        ("communication", "inbox") => Some(communication_inbox_pack_definition()),
        ("communication", "calendar") => Some(communication_calendar_pack_definition()),
        ("knowledge", "search") => Some(knowledge_search_pack_definition()),
        ("knowledge", "retrieval") => Some(knowledge_retrieval_pack_definition()),
        ("knowledge", "document-parsing") => Some(knowledge_document_parsing_pack_definition()),
        ("knowledge", "citations") => Some(knowledge_citations_pack_definition()),
        ("knowledge", "graph") => Some(knowledge_graph_pack_definition()),
        ("knowledge", "summarization") => Some(knowledge_summarization_pack_definition()),
        ("developer", "code") => Some(developer_code_pack_definition()),
        ("developer", "repository") => Some(developer_repository_pack_definition()),
        ("developer", "ci") => Some(developer_ci_pack_definition()),
        ("developer", "issue-tracker") => Some(developer_issue_tracker_pack_definition()),
        ("developer", "terminal") => Some(developer_terminal_pack_definition()),
        ("developer", "browser-automation") => Some(developer_browser_automation_pack_definition()),
        ("developer", "design-tools") => Some(developer_design_tools_pack_definition()),
        ("office", "document") => Some(office_document_pack_definition()),
        ("office", "spreadsheet") => Some(office_spreadsheet_pack_definition()),
        ("office", "presentation") => Some(office_presentation_pack_definition()),
        ("office", "pdf") => Some(office_pdf_pack_definition()),
        ("office", "forms") => Some(office_forms_pack_definition()),
        ("media", "image") => Some(media_image_pack_definition()),
        ("media", "audio") => Some(media_audio_pack_definition()),
        ("media", "video") => Some(media_video_pack_definition()),
        ("media", "transcription") => Some(media_transcription_pack_definition()),
        ("media", "rendering") => Some(media_rendering_pack_definition()),
        ("finance", "market-data") => Some(finance_market_data_pack_definition()),
        ("finance", "stock") => Some(finance_stock_pack_definition()),
        ("finance", "crypto") => Some(finance_crypto_pack_definition()),
        ("finance", "accounting") => Some(finance_accounting_pack_definition()),
        ("finance", "portfolio") => Some(finance_portfolio_pack_definition()),
        ("finance", "invoice") => Some(finance_invoice_pack_definition()),
        ("commerce", "catalog") => Some(commerce_catalog_pack_definition()),
        ("commerce", "cart") => Some(commerce_cart_pack_definition()),
        ("commerce", "order") => Some(commerce_order_pack_definition()),
        ("commerce", "payment-intent") => Some(commerce_payment_intent_pack_definition()),
        ("commerce", "receipt") => Some(commerce_receipt_pack_definition()),
        ("commerce", "entitlement") => Some(commerce_entitlement_pack_definition()),
        ("identity", "account") => Some(identity_account_pack_definition()),
        ("identity", "profile") => Some(identity_profile_pack_definition()),
        ("identity", "auth-handoff") => Some(identity_auth_handoff_pack_definition()),
        ("identity", "organization") => Some(identity_organization_pack_definition()),
        ("identity", "tenant") => Some(identity_tenant_pack_definition()),
        ("location", "maps") => Some(location_maps_pack_definition()),
        ("location", "geocode") => Some(location_geocode_pack_definition()),
        ("location", "route") => Some(location_route_pack_definition()),
        ("location", "place-search") => Some(location_place_search_pack_definition()),
        ("location", "timezone") => Some(location_timezone_pack_definition()),
        ("device", "sensors") => Some(device_sensors_pack_definition()),
        ("device", "camera") => Some(device_camera_pack_definition()),
        ("device", "local-files") => Some(device_local_files_pack_definition()),
        ("device", "notifications") => Some(device_notifications_pack_definition()),
        ("device", "foreground-background-host") => {
            Some(device_foreground_background_host_pack_definition())
        }
        ("ai", "llm") => Some(ai_llm_pack_definition()),
        ("ai", "embedding") => Some(ai_embedding_pack_definition()),
        ("ai", "rerank") => Some(ai_rerank_pack_definition()),
        ("ai", "vision") => Some(ai_vision_pack_definition()),
        ("ai", "speech") => Some(ai_speech_pack_definition()),
        ("ai", "model-evaluation") => Some(ai_model_evaluation_pack_definition()),
        ("workflow", "task") => Some(workflow_task_pack_definition()),
        ("workflow", "schedule") => Some(workflow_schedule_pack_definition()),
        ("workflow", "approval") => Some(workflow_approval_pack_definition()),
        ("workflow", "delegation") => Some(workflow_delegation_pack_definition()),
        ("workflow", "review") => Some(workflow_review_pack_definition()),
        ("workflow", "recovery") => Some(workflow_recovery_pack_definition()),
        _ => None,
    }
}

fn unavailable_sub_pack_definition(
    family_id: &'static str,
    slug: &'static str,
    label: &'static str,
) -> DomainPackDefinition {
    let pack_id = format!("pack.{family_id}.{slug}.v1");
    let parent_pack_id = format!("pack.{family_id}.v1");
    DomainPackDefinition::with_metadata(
        pack_id,
        DomainPackMetadata {
            family_id: family_id.into(),
            parent_pack_id: Some(parent_pack_id),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            permission_scopes: BTreeSet::from([format!("pack.{family_id}.{slug}.discover")]),
            source_attribution: BTreeSet::from([
                "openspec:add-developer-pack-industrial-capability-catalog".into(),
                format!("openspec:add-pack-{family_id}-{slug}"),
            ]),
            migration_notes: vec![format!(
                "{label} is discoverable in the industrial catalog and becomes callable only after an approved serviceized provider registers concrete command schemas."
            )],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(30_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: "descriptor_only".into(),
                retention_policy: "catalog_metadata_only".into(),
                redaction_policy: "no_provider_payloads".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: format!("sdk.packs.{family_id}.{slug}"),
                docs_url: format!("docs://macaca/developer-packs/{family_id}/{slug}"),
                examples: vec![format!(
                    "Declare `pack.{family_id}.{slug}.v1` as optional until a provider marks it available."
                )],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "catalog.descriptor".into(),
                unavailable_reason: "industrial_pack_provider_not_installed".into(),
                replay_schema: "pack.discovery.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::new(),
            },
            ..Default::default()
        },
        [],
    )
}
