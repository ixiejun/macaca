//! `macaca-runtime-host` — Agent OS runtime host.
//!
//! This crate owns OS-level runtime glue that is independent of any single
//! Agent OS host (HTTP, CLI, gateway, background schedulers). It contains:
//!
//! - [`mcp_runtime`] — MCP registry, runtime manager and per-scope lifecycle
//! - [`skill_mcp_mapping_registry`] — declarative skill install spec → MCP
//!   server mappings (no product-name hardcoding in control flow)
//!
//! Framework protocol handling stays in [`macaca_framework::mcp`]; this crate
//! provides the Agent OS-level registry, policy, status and toolkit
//! registration layered on top.

pub mod agent_context_service_provider;
pub mod agent_execution_dispatch;
pub mod agent_execution_orchestration;
pub mod agent_execution_ports;
pub mod agent_execution_service_provider;
pub mod alert_service_provider;
pub(crate) mod app_protocol_service_commands;
pub mod app_protocol_service_provider;
mod app_public_api;
mod application_execution_event_builder;
mod application_execution_event_store;
mod application_execution_external_backend;
mod application_execution_external_backend_diagnostics;
mod application_execution_external_backend_results;
mod application_execution_external_backend_transport;
mod application_execution_gateway_events;
mod application_execution_hosted;
mod application_execution_projection;
mod application_execution_provider_registry;
mod application_execution_remote_agent;
mod application_execution_service_host;
mod application_execution_service_logs;
pub mod application_execution_service_provider;
mod application_execution_service_snapshots;
pub mod application_hosts;
pub mod application_service_provider;
pub(crate) mod approval_service_commands;
pub mod approval_service_provider;
pub mod autonomy_dispatch;
pub mod autonomy_evolution_live_executor;
pub mod autonomy_evolution_service_provider;
pub(crate) mod autonomy_result_evidence;
pub mod autonomy_runtime_config;
pub mod autonomy_service_provider;
pub mod autonomy_supervisor;
pub mod calendar_service_provider;
pub mod citation_service_provider;
pub mod code_intelligence_service_provider;
pub mod commerce_cart_service_provider;
mod commerce_cart_strategy;
pub mod commerce_catalog_service_provider;
pub mod commerce_catalog_strategy;
pub mod commerce_entitlement_service_provider;
pub mod commerce_order_service_provider;
pub mod commerce_order_strategy;
pub mod commerce_payment_intent_service_provider;
pub mod commerce_payment_intent_strategy;
pub mod commerce_receipt_service_provider;
pub mod composed_agent_execution_backend;
pub(crate) mod config_service_commands;
pub mod config_service_provider;
pub mod context_service_provider;
pub mod delegated_task_dispatcher;
pub mod developer_browser_automation_service_provider;
pub mod developer_browser_automation_strategy;
pub mod developer_ci_service_provider;
pub mod developer_ci_strategy;
pub mod developer_code_service_provider;
pub mod developer_code_strategy;
pub mod developer_design_tools_service_provider;
pub mod developer_design_tools_strategy;
pub mod developer_issue_tracker_service_provider;
pub mod developer_issue_tracker_strategy;
pub mod developer_repository_service_provider;
pub mod developer_repository_strategy;
pub mod developer_terminal_service_provider;
pub mod developer_terminal_strategy;
pub mod device_camera_service_provider;
mod device_camera_service_state;
pub mod device_host_lifecycle_service_provider;
pub mod device_local_files_service_provider;
pub mod device_sensors_service_provider;
pub mod diagnostics_service_provider;
pub mod document_parsing_service_provider;
mod domain_pack_provider_replacement;
pub mod domain_pack_service_provider;
pub mod domain_pack_simple_provider;
pub mod driver_service_bootstrap;
pub mod driver_service_provider;
pub mod email_service_provider;
pub mod entitlement;
pub mod entitlement_service_provider;
pub mod env_bridge;
pub mod evm_service_provider;
pub mod execution_control;
pub mod execution_control_fork_join;
pub mod execution_control_goal_lifecycle;
pub mod execution_control_local_notification;
pub mod execution_control_runtime;
pub mod execution_control_service_provider;
pub mod execution_control_session_loop;
/// Event-driven agent task execution runtime (queue, fork-join, worker dispatch).
///
/// Evicted from `macaca-kernel` during P2 microkernel purification. Execution orchestration
/// is a runtime-host concern: it composes kernel persistence/logging ports with service-backed
/// agent runners while keeping the microkernel limited to scheduling invariants.
pub mod executor;
pub mod factory;
pub(crate) mod file_service_local;
pub mod file_service_provider;
pub mod finance_accounting_service_provider;
mod finance_accounting_strategy;
pub mod finance_crypto_service_provider;
pub mod finance_crypto_strategy;
pub mod finance_invoice_service_provider;
pub mod finance_market_data_service_provider;
pub mod finance_market_data_strategy;
pub mod finance_portfolio_service_provider;
pub mod finance_stock_service_provider;
pub mod location_geocode_service_provider;
pub mod location_maps_service_provider;
pub mod location_route_service_provider;
pub mod media_image_service_provider;
pub mod media_rendering_service_provider;
pub mod media_video_service_provider;
pub mod office_document_service_provider;
pub mod office_forms_service_provider;
pub mod office_pdf_service_provider;
pub mod office_presentation_service_provider;
pub mod office_spreadsheet_service_provider;
pub mod commerce_entitlement_strategy;
pub mod commerce_receipt_strategy;
pub mod finance_invoice_strategy;
pub mod finance_portfolio_strategy;
pub mod finance_stock_strategy;
pub mod location_geocode_strategy;
pub mod location_maps_strategy;
pub mod location_route_strategy;
pub mod media_image_strategy;
pub mod media_rendering_strategy;
pub mod media_video_strategy;
pub mod office_document_strategy;
pub mod office_forms_strategy;
pub mod office_pdf_strategy;
pub mod office_presentation_strategy;
pub mod office_spreadsheet_strategy;
pub use domain_pack_simple_provider::{
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
};
pub mod foundation_config_service_provider;
pub mod foundation_filesystem_service_provider;
pub mod foundation_key_value_state_service_provider;
pub mod foundation_secrets_reference_service_provider;
pub mod foundation_session_state_embedded_provider;
pub mod foundation_session_state_remote_bridge;
pub mod foundation_session_state_service_provider;
mod framework_public_api;
pub mod framework_runtime_agent_service;
pub mod genui_surface_store;
pub mod git_service_provider;
pub mod graph_service_provider;
pub(crate) mod hook_service_commands;
pub mod hook_service_provider;
pub mod identity_account_service_provider;
mod identity_account_strategy;
pub mod identity_auth_handoff_service_provider;
pub mod identity_organization_service_provider;
pub mod identity_profile_service_provider;
mod identity_profile_strategy;
pub mod identity_tenant_service_provider;
pub mod inbox_service_provider;
pub mod interaction_ledger_store;
pub mod interaction_service_bootstrap;
pub(crate) mod interaction_service_items;
pub mod interaction_service_provider;
pub(crate) mod interaction_service_threads;
pub(crate) mod interaction_service_turns;
pub mod lease;
pub mod llm_service_catalog;
pub mod llm_service_hardening;
pub mod llm_service_provider;
pub mod location_place_search_service_provider;
mod location_place_search_strategy;
pub mod location_timezone_service_provider;
mod location_timezone_service_provider_tests;
mod location_timezone_strategy;
pub(crate) mod mcp_descriptor_index;
pub(crate) mod mcp_invocation_registry;
pub(crate) mod mcp_operator_lifecycle;
pub mod mcp_runtime;
pub mod mcp_service_provider;
pub mod media_audio_service_provider;
mod media_audio_service_state;
mod media_audio_strategy;
pub mod media_transcription_service_provider;
mod media_transcription_service_state;
mod media_transcription_strategy;
pub mod memory_service_provider;
pub mod messaging_service_provider;
pub mod notification_service_provider;
pub mod optional_service_bootstrap;
pub mod package;
pub mod payment_adapter;
pub mod payment_admission;
pub mod payment_policy;
pub mod payment_service_provider;
pub use payment_service_provider::payment_service_descriptor;
#[cfg(test)]
mod device_camera_service_provider_tests;
#[cfg(test)]
mod device_host_lifecycle_service_provider_tests;
#[cfg(test)]
mod device_local_files_service_provider_tests;
#[cfg(test)]
mod device_sensors_service_provider_tests;
#[cfg(test)]
mod foundation_config_service_audit_tests;
#[cfg(test)]
mod foundation_filesystem_service_audit_tests;
mod foundation_key_value_state_audit_tests;
#[cfg(test)]
mod foundation_secrets_reference_audit_tests;
#[cfg(test)]
mod foundation_session_state_embedded_provider_tests;
#[cfg(test)]
mod foundation_session_state_service_provider_tests;
mod local_files_policy;
#[cfg(test)]
mod media_audio_service_provider_tests;
mod media_transcription_policy_tests;
#[cfg(test)]
mod media_transcription_service_provider_tests;
pub mod plugin;
pub mod plugin_capability;
pub mod plugin_capability_service_provider;
pub mod plugin_control;
pub mod plugin_control_service_provider;
pub mod plugin_hook;
pub mod plugin_hook_service_provider;
pub mod plugin_hosts;
pub(crate) mod plugin_marketplace_service_commands;
pub mod plugin_marketplace_service_provider;
pub(crate) mod plugin_marketplace_service_support;
pub(crate) mod plugin_marketplace_snapshot_decode;
pub(crate) mod process_service_local;
pub mod process_service_provider;
pub(crate) mod process_service_records;
#[cfg(test)]
mod random_service_audit_tests;
pub mod random_service_provider;
pub mod realtime_service_provider;
pub mod remote_environment_service_provider;
pub mod retrieval_service_provider;
pub mod review_service_provider;
pub(crate) mod sandbox_service_local;
pub mod sandbox_service_provider;
pub mod search_service_provider;
pub mod service_audit_runtime_bundle;
pub mod service_call_audit;
pub mod service_call_audit_service_provider;
pub mod service_contract_registry;
pub mod service_decorator;
pub mod service_policy_engine;
pub mod service_provider;
pub mod service_provider_selector;
pub mod service_router;
pub mod service_runtime;
pub mod service_runtime_error;
pub mod service_runtime_event;
pub mod session_loop_local_runtime;
pub(crate) mod skill_alias_resolution;
pub mod skill_bootstrap;
pub mod skill_mcp_mapping_registry;
pub(crate) mod skill_operator_lifecycle;
mod skill_public_api;
pub(crate) mod skill_service_codec;
pub(crate) mod skill_service_content_mutation;
pub(crate) mod skill_service_experience_routing;
pub(crate) mod skill_service_governance_store;
pub mod skill_service_provider;
pub(crate) mod skill_service_provider_curation;
pub(crate) mod skill_service_provider_curation_log;
pub(crate) mod skill_service_provider_event_journal;
pub(crate) mod skill_service_provider_lifecycle;
pub(crate) mod skill_service_provider_materialization_operator;
pub(crate) mod skill_service_provider_merge;
pub(crate) mod skill_service_provider_package_recovery;
pub(crate) mod skill_service_provider_proposal_materialization;
pub(crate) mod skill_service_provider_proposal_processing;
pub(crate) mod skill_service_provider_proposals;
pub(crate) mod skill_service_provider_semantic_review;
pub(crate) mod skill_service_provider_state;
pub mod store_entitlement_admission;
pub mod store_service_provider;
pub mod summarization_service_provider;
pub mod summarization_strategy;
pub mod task_service_provider;
pub mod task_toolkit_bootstrap;
pub mod time_service_provider;

#[cfg(test)]
mod time_service_audit_tests;
pub mod tool_bootstrap;
pub mod tool_family_providers;
pub mod tool_service_availability;
pub mod tool_service_environment;
pub mod tool_service_gateway;
pub mod tool_service_invocation;
pub mod tool_service_planning;
pub mod tool_service_provider;
pub mod tool_service_provider_state;
pub mod tool_service_result;
pub mod transport;
pub mod wasm_runtime_provider;
pub mod web3_service_provider;
pub mod workflow_task_service_provider;
pub mod workspace_toolkit_bootstrap;

#[cfg(test)]
mod app_protocol_service_provider_tests;
#[cfg(test)]
mod application_execution_event_store_tests;
#[cfg(test)]
mod application_execution_external_backend_e2e_tests;
#[cfg(test)]
mod application_execution_external_backend_tests;
#[cfg(test)]
mod application_execution_gateway_service_tests;
#[cfg(test)]
mod application_execution_hosted_tests;
#[cfg(test)]
mod application_execution_provider_registry_tests;
#[cfg(test)]
mod application_execution_remote_agent_tests;
#[cfg(test)]
mod application_execution_service_provider_tests;
#[cfg(test)]
mod approval_service_provider_tests;
#[cfg(test)]
mod calendar_service_provider_tests;
#[cfg(test)]
mod citation_service_provider_tests;
#[cfg(test)]
mod code_intelligence_service_provider_tests;
#[cfg(test)]
mod commerce_cart_service_provider_tests;
mod commerce_catalog_service_provider_tests;
mod commerce_order_service_provider_tests;
mod commerce_payment_intent_service_provider_tests;
#[cfg(test)]
mod config_service_provider_tests;
mod developer_browser_automation_service_provider_tests;
mod developer_ci_service_provider_tests;
mod developer_code_service_provider_tests;
mod developer_design_tools_service_provider_tests;
mod developer_issue_tracker_service_provider_tests;
mod developer_repository_service_provider_tests;
mod developer_terminal_service_provider_tests;
#[cfg(test)]
mod diagnostics_realtime_remote_service_provider_tests;
#[cfg(test)]
mod document_parsing_service_provider_tests;
#[cfg(test)]
mod email_service_provider_tests;
#[cfg(test)]
mod file_service_provider_tests;
mod finance_accounting_service_provider_tests;
mod finance_crypto_service_provider_tests;
mod finance_market_data_service_provider_tests;
#[cfg(test)]
mod git_service_provider_tests;
#[cfg(test)]
mod graph_service_provider_tests;
mod graph_strategies;
#[cfg(test)]
mod graph_strategies_tests;
#[cfg(test)]
mod hook_service_provider_tests;
#[cfg(test)]
mod identity_account_service_provider_tests;
#[cfg(test)]
mod identity_auth_handoff_service_provider_tests;
mod identity_auth_handoff_strategy;
#[cfg(test)]
mod identity_auth_handoff_strategy_tests;
#[cfg(test)]
mod identity_governance_service_provider_tests;
#[cfg(test)]
mod identity_profile_service_provider_tests;
#[cfg(test)]
mod inbox_service_provider_tests;
#[cfg(test)]
mod interaction_service_provider_tests;
#[cfg(test)]
mod interaction_service_state_tests;
#[cfg(test)]
mod llm_service_provider_hardening_tests;
mod location_place_search_service_provider_tests;
#[cfg(test)]
mod messaging_service_provider_tests;
#[cfg(test)]
mod notification_service_provider_lifecycle_tests;
#[cfg(test)]
mod notification_service_provider_tests;
#[cfg(test)]
mod plugin_marketplace_service_provider_tests;
#[cfg(test)]
mod process_service_provider_tests;
#[cfg(test)]
mod retrieval_service_provider_tests;
#[cfg(test)]
mod review_service_provider_tests;
#[cfg(test)]
mod sandbox_service_provider_tests;
#[cfg(test)]
mod search_service_provider_tests;
#[cfg(test)]
mod service_router_tests;
#[cfg(test)]
mod skill_content_mutation_tests;
#[cfg(test)]
mod skill_governance_store_logging_tests;
#[cfg(test)]
mod skill_materialization_operator_tests;
#[cfg(test)]
mod skill_operator_lifecycle_tests;
#[cfg(test)]
mod skill_optional_provider_boundary_tests;
#[cfg(test)]
mod skill_proposal_lifecycle_tests;
#[cfg(test)]
mod skill_proposal_materialization_tests;
#[cfg(test)]
mod skill_proposal_processing_tests;
#[cfg(test)]
mod skill_sanitization_boundary_tests;
#[cfg(test)]
mod skill_self_evolution_evaluation_harness_fixture;
#[cfg(test)]
mod skill_self_evolution_evaluation_harness_tests;
#[cfg(test)]
mod skill_service_lifecycle_tests;
#[cfg(test)]
mod skill_service_merge_tests;
#[cfg(test)]
mod skill_service_provider_tests;
#[cfg(test)]
mod skill_service_usage_tests;
#[cfg(test)]
mod summarization_service_provider_tests;
#[cfg(test)]
mod summarization_strategy_tests;
#[cfg(test)]
mod tool_service_audit_tests;
#[cfg(test)]
mod tool_service_environment_tests;
#[cfg(test)]
mod tool_service_family_provider_tests;
#[cfg(test)]
mod tool_service_gateway_tests;
#[cfg(test)]
mod tool_service_invocation_tests;
#[cfg(test)]
mod tool_service_planning_tests;
#[cfg(test)]
mod unified_agent_execution_provider_tests;
#[cfg(test)]
mod unified_audit_replay_convergence_tests;
#[cfg(test)]
mod workflow_task_service_provider_tests;

mod runtime_host_public_api;
pub use runtime_host_public_api::*;
