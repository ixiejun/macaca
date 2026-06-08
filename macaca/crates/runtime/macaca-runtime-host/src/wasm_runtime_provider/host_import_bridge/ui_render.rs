//! GenUI `ui.render` import handling via the Repository pattern.
//!
//! Presentation imports bypass ServiceRuntime routing. The bridge validates the
//! declarative intent schema, hydrates host-owned scope, and persists the intent
//! in the shared GenUI surface store for later Application Service queries.

use macaca_app::UiIntentValidator;
use macaca_proto::{
    ApplicationHostCommandResult, TraceContext, UiIntent, WasmHostImportCommand,
    WasmHostImportErrorKind,
};
use tracing::{info, warn};

use super::routing_support::hydrate_ui_render_scope;
use super::WasmHostImportBridge;

impl WasmHostImportBridge {
    /// Store a declarative GenUI render intent without routing through services.
    ///
    /// `ui.render` is a presentation import, not a data/service import. The
    /// bridge validates the schema and writes it to the shared repository so the
    /// shell can query it later through the Application Service. This keeps UI
    /// rendering generic, traceable, and independent from application-specific
    /// code paths.
    pub(super) async fn dispatch_ui_render(
        &self,
        guest_command: WasmHostImportCommand,
        trace: TraceContext,
    ) -> ApplicationHostCommandResult {
        let Some(store) = self.genui_surface_store.as_ref() else {
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::ServiceUnavailable,
                "WASM ui.render store is unavailable",
            );
        };
        let mut intent = match serde_json::from_value::<UiIntent>(guest_command.payload.clone()) {
            Ok(intent) => intent,
            Err(error) => {
                warn!(
                    trace_id = %trace.trace_id,
                    error = %error,
                    "WASM ui.render payload failed to decode"
                );
                return self.denied_result(
                    &guest_command,
                    WasmHostImportErrorKind::PolicyDenied,
                    "WASM ui.render payload is invalid",
                );
            }
        };
        hydrate_ui_render_scope(&mut intent, &guest_command, &trace);
        if let Err(error) = UiIntentValidator.validate_intent(&intent) {
            warn!(
                trace_id = %trace.trace_id,
                error = %error,
                "WASM ui.render intent failed validation"
            );
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::PolicyDenied,
                "WASM ui.render intent failed validation",
            );
        }
        if let Err(error) = store.store(intent.clone()).await {
            warn!(
                trace_id = %trace.trace_id,
                error = %error,
                "WASM ui.render intent failed to store"
            );
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::ServiceFailed,
                "WASM ui.render store failed",
            );
        }
        let mut result =
            ApplicationHostCommandResult::ok(serde_json::json!({ "stored": true }), Some(trace));
        result
            .metadata
            .insert("reason_code".into(), "ui_render_stored".into());
        result
            .metadata
            .insert("surface_id".into(), intent.surface_id.to_string());
        self.attach_common_metadata(&guest_command, &mut result);
        info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            "WASM ui.render intent stored"
        );
        result
    }
}
