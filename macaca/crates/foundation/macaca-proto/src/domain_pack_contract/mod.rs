//! Provider-neutral domain-pack contract shared across application, package, and shell layers.
//!
//! Domain packs are declarative capability bundles.  Applications request packs through
//! manifest service contracts; optional package crates publish metadata and concrete service
//! providers separately.  This module owns only the data contracts, deterministic expansion,
//! validation specifications, and trace-safe adapter helpers.
//!
//! # Design patterns
//! - **Value Object**: pack metadata structs are immutable DTOs.
//! - **Registry**: the in-memory catalog provides deterministic lookup.
//! - **Strategy**: the catalog trait allows alternate stores without changing expansion.
//! - **Specification**: validators make taxonomy rules executable and auditable.
//! - **Composition Root**: catalog composition happens once at host bootstrap.

pub mod ai_common;
pub mod ai_embedding;
pub mod ai_llm;
pub mod ai_model_evaluation;
pub mod ai_preflight;
pub mod ai_rerank;
pub mod ai_speech;
pub mod ai_vision;
mod catalog;
pub mod commerce_cart;
mod commerce_cart_validation;
pub mod commerce_catalog;
mod commerce_catalog_hashes;
mod commerce_catalog_validation;
pub mod commerce_common;
pub mod commerce_entitlement;
mod commerce_entitlement_validation;
pub mod commerce_order;
mod commerce_order_validation;
pub mod commerce_payment_intent;
mod commerce_payment_intent_validation;
pub mod commerce_receipt;
mod commerce_receipt_validation;
mod communication_calendar;
pub mod communication_calendar_preflight;
mod communication_calendar_validation;
mod communication_common;
mod communication_email;
pub mod communication_email_preflight;
mod communication_email_validation;
mod communication_inbox;
pub mod communication_inbox_preflight;
mod communication_inbox_validation;
mod communication_messaging;
pub mod communication_messaging_preflight;
mod communication_messaging_validation;
mod communication_notification;
pub mod communication_notification_preflight;
mod communication_notification_validation;
pub mod developer_browser_automation;
pub mod developer_ci;
pub mod developer_code;
pub mod developer_common;
pub mod developer_design_tools;
pub mod developer_issue_tracker;
pub mod developer_repository;
pub mod developer_terminal;
pub mod device_camera;
pub mod device_camera_preflight;
mod device_camera_validation;
pub mod device_common;
pub mod device_foreground_background_host;
pub mod device_host_lifecycle_preflight;
mod device_host_lifecycle_validation;
pub mod device_local_files;
mod device_local_files_validation;
pub mod device_notifications;
pub mod device_sensors;
mod device_validation;
mod expansion;
mod exports;
mod exports_foundation_filesystem;
pub mod finance_accounting;
mod finance_accounting_bounds;
mod finance_accounting_commands;
mod finance_accounting_hashes;
mod finance_accounting_model;
mod finance_accounting_preflight;
mod finance_accounting_reports;
pub mod finance_common;
pub mod finance_crypto;
pub mod finance_invoice;
mod finance_invoice_validation;
pub mod finance_market_data;
pub mod finance_portfolio;
pub mod finance_portfolio_async;
mod finance_portfolio_validation;
pub mod finance_stock;
mod foundation_config;
pub mod foundation_config_semantics;
mod foundation_config_validation;
mod foundation_filesystem;
pub mod foundation_filesystem_semantics;
mod foundation_filesystem_validation;
mod foundation_key_value_state;
pub mod foundation_key_value_state_semantics;
mod foundation_key_value_state_validation;
mod foundation_random;
pub mod foundation_random_semantics;
mod foundation_random_validation;
mod foundation_secrets_reference;
mod foundation_secrets_reference_manifest_validation;
pub mod foundation_secrets_reference_semantics;
mod foundation_secrets_reference_validation;
mod foundation_session_state;
pub mod foundation_session_state_semantics;
mod foundation_session_state_validation;
mod foundation_time;
pub mod foundation_time_semantics;
mod foundation_time_validation;
mod foundation_validation;
pub mod identity_account;
mod identity_account_validation;
mod identity_account_validation_permissions;
pub mod identity_auth_handoff;
mod identity_auth_handoff_validation;
pub mod identity_common;
pub mod identity_organization;
pub mod identity_organization_semantics;
pub mod identity_profile;
mod identity_profile_validation;
mod identity_profile_validation_permissions;
pub mod identity_tenant;
pub mod identity_tenant_semantics;
mod identity_validation;
mod industrial_pack_taxonomy;
mod industrial_reference_catalogs;
mod knowledge_citations;
mod knowledge_citations_preflight;
mod knowledge_common;
mod knowledge_document_parsing;
mod knowledge_document_parsing_preflight;
mod knowledge_graph;
mod knowledge_graph_validation;
mod knowledge_retrieval;
mod knowledge_retrieval_preflight;
mod knowledge_search;
mod knowledge_search_preflight;
mod knowledge_summarization;
mod knowledge_summarization_preflight;
pub mod location_common;
pub mod location_geocode;
pub mod location_maps;
pub mod location_place_search;
pub mod location_route;
pub mod location_timezone;
pub mod media_audio;
pub mod media_audio_preflight;
mod media_audio_validation;
pub mod media_common;
pub mod media_image;
pub mod media_rendering;
pub mod media_transcription;
pub mod media_transcription_preflight;
pub mod media_transcription_semantics;
mod media_transcription_validation;
pub mod media_video;
mod model;
mod model_diagnostics;
pub mod office_common;
pub mod office_document;
pub mod office_forms;
pub mod office_pdf;
pub mod office_presentation;
pub mod office_spreadsheet;
pub mod pack_preflight;
mod reference_catalogs;
mod service_helpers;
mod spec;
pub mod workflow_approval;
pub mod workflow_approval_semantics;
pub mod workflow_common;
pub mod workflow_delegation;
pub mod workflow_delegation_semantics;
pub mod workflow_recovery;
pub mod workflow_recovery_semantics;
pub mod workflow_review;
pub mod workflow_review_semantics;
pub mod workflow_schedule;
pub mod workflow_schedule_semantics;
pub mod workflow_task;
pub mod workflow_task_approval_spec;
pub mod workflow_task_dispatch_gate;
pub mod workflow_task_lifecycle_event;
pub mod workflow_task_lifecycle_spec;
pub mod workflow_task_resource_spec;
pub mod workflow_task_transition;

#[cfg(test)]
mod foundation_config_semantics_tests;
#[cfg(test)]
mod foundation_filesystem_semantics_tests;
#[cfg(test)]
mod foundation_random_semantics_tests;
#[cfg(test)]
mod identity_organization_semantics_tests;
#[cfg(test)]
mod identity_tenant_semantics_tests;
#[cfg(test)]
mod knowledge_citations_preflight_tests;
#[cfg(test)]
mod knowledge_document_parsing_preflight_tests;
#[cfg(test)]
mod knowledge_retrieval_preflight_tests;
#[cfg(test)]
mod knowledge_search_preflight_tests;
#[cfg(test)]
mod knowledge_summarization_preflight_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workflow_approval_semantics_tests;
#[cfg(test)]
mod workflow_delegation_semantics_tests;
#[cfg(test)]
mod workflow_recovery_semantics_tests;
#[cfg(test)]
mod workflow_review_preflight_tests;
#[cfg(test)]
mod workflow_review_semantics_tests;
#[cfg(test)]
mod workflow_schedule_semantics_tests;

pub use exports::*;
