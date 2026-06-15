//! WASM host import bridge backed by `ServiceRuntime` (Facade module tree).
//!
//! This module is the Bridge between guest-facing Application ABI imports and
//! Macaca's provider-neutral service runtime. It owns only translation,
//! validation, routing, result bounding, and audit logging. It intentionally
//! does not know concrete service implementations, workflow names, driver
//! names, backend clients, or business behavior.
//!
//! Module layout (Facade + Bridge pattern):
//! - `constants.rs` — provider-neutral service ids and operation names
//! - `construction.rs` — bridge construction, fluent configuration, audit replay API
//! - `dispatch.rs` — primary Command dispatch entry for admitted imports
//! - `task_lifecycle.rs` — Task Service Adapter for `agent_delegate` durability
//! - `ui_render.rs` — GenUI Repository path for declarative `ui.render` intents
//! - `policy_validation.rs` — admission validation and app-scoped policy overrides
//! - `result_shaping.rs` — denied/error result shaping and metadata attachment
//! - `routing_support.rs` — pure Adapter helpers for payload hydration and sanitization

mod constants;
mod construction;
mod dispatch;
mod policy_validation;
mod result_shaping;
mod routing_support;
mod task_lifecycle;
mod ui_render;

use std::fmt;
use std::sync::Arc;

use macaca_proto::{ServiceBusSource, DEFAULT_WASM_MAX_PAYLOAD_BYTES};

use crate::{
    ApplicationGenUiSurfaceStore, InMemoryServicePolicyEngine, ServiceCallAuditSink, ServiceRouter,
};

use super::telemetry::WasmTelemetrySinkRef;

pub use constants::APPLICATION_EXECUTION_GRAPH_OWNER;

/// Runtime configuration for the host import bridge.
///
/// Host composition layers supply bounded payload limits and a stable bus source
/// label so audit and telemetry consumers can correlate WASM imports without
/// inspecting guest bytecode or application manifests.
#[derive(Clone)]
pub struct WasmHostImportBridgeConfig {
    pub source: ServiceBusSource,
    pub max_payload_bytes: u64,
    pub max_output_bytes: u64,
}

impl Default for WasmHostImportBridgeConfig {
    fn default() -> Self {
        Self {
            source: ServiceBusSource::new("wasm.host.import"),
            max_payload_bytes: DEFAULT_WASM_MAX_PAYLOAD_BYTES,
            max_output_bytes: DEFAULT_WASM_MAX_PAYLOAD_BYTES,
        }
    }
}

/// Bridge that validates and routes WASM host imports through `ServiceRuntime`.
///
/// The struct keeps routing dependencies as `pub(super)` fields so sibling
/// modules can implement focused `impl` blocks without exposing internals to
/// the wider runtime-host crate.
#[derive(Clone)]
pub struct WasmHostImportBridge {
    pub(super) router: Arc<ServiceRouter>,
    pub(super) policy_engine: Arc<InMemoryServicePolicyEngine>,
    pub(super) audit_sink: Arc<dyn ServiceCallAuditSink>,
    pub(super) config: WasmHostImportBridgeConfig,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
    pub(super) genui_surface_store: Option<ApplicationGenUiSurfaceStore>,
}

impl fmt::Debug for WasmHostImportBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmHostImportBridge")
            .field("source", &self.config.source.to_string())
            .field("max_payload_bytes", &self.config.max_payload_bytes)
            .field("max_output_bytes", &self.config.max_output_bytes)
            .finish_non_exhaustive()
    }
}
