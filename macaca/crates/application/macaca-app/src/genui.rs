//! Application Framework GenUI Runtime v0.
//!
//! This module validates and packages declarative UI intents before they cross
//! into presentation shells.  It uses a small Facade plus Specification-style
//! validator so application code can emit UI without depending on `macaca-web`
//! or frontend renderer internals.

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationImport, TraceContext,
    UiComponent, UiComponentKind, UiEventCommand, UiIntent, UiRenderError,
};
use serde_json::json;
use tracing::{info, warn};

/// Result alias for GenUI validation and runtime operations.
pub type GenUiResult<T> = Result<T, UiRenderError>;

/// GenUI validator that traverses component trees like a Visitor.
#[derive(Debug, Default, Clone)]
pub struct UiIntentValidator;

impl UiIntentValidator {
    /// Validate a render intent before it reaches a renderer or EventLog.
    pub fn validate_intent(&self, intent: &UiIntent) -> GenUiResult<()> {
        info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            "GenUI intent validation started"
        );
        require_scope(&intent.app_id, &intent.session_id, intent.trace.as_ref())?;
        self.visit_component(&intent.tree.root)?;
        info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            "GenUI intent validation passed"
        );
        Ok(())
    }

    /// Validate an event command before it is persisted or dispatched.
    pub fn validate_event(&self, command: &UiEventCommand) -> GenUiResult<()> {
        let event = &command.event;
        info!(
            app_id = %event.app_id,
            session_id = %event.session_id,
            surface_id = %event.surface_id,
            component_id = %event.component_id,
            action = %event.action.name,
            "GenUI event command validation started"
        );
        require_scope(&event.app_id, &event.session_id, event.trace.as_ref())?;
        Ok(())
    }

    /// Traverse one component and all children.
    fn visit_component(&self, component: &UiComponent) -> GenUiResult<()> {
        if !component.kind.is_supported_v0() {
            warn!(
                component_id = %component.id,
                component_kind = %component.kind,
                "GenUI component rejected because kind is unsupported"
            );
            return Err(UiRenderError::UnsupportedComponent(
                component.kind.to_string(),
            ));
        }
        reject_script_like_payload(&component.props)?;
        for child in &component.children {
            self.visit_component(child)?;
        }
        Ok(())
    }
}

/// Application-facing GenUI runtime facade.
#[derive(Debug, Default, Clone)]
pub struct GenUiRuntime {
    validator: UiIntentValidator,
}

impl GenUiRuntime {
    /// Create a runtime with the default validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate an intent and convert it into an ABI `macaca:ui/render` command.
    pub fn render_command(
        &self,
        intent: UiIntent,
        trace: TraceContext,
    ) -> GenUiResult<ApplicationHostCommand> {
        self.validator.validate_intent(&intent)?;
        let payload = serde_json::to_value(&intent)
            .map_err(|error| UiRenderError::UnsafePayload(error.to_string()))?;
        info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            "GenUI render command created"
        );
        Ok(ApplicationHostCommand::with_trace(
            ApplicationImport::UiRender,
            payload,
            trace,
        ))
    }

    /// Validate a UI event command before web routes persist it.
    pub fn validate_event(&self, command: &UiEventCommand) -> GenUiResult<()> {
        self.validator.validate_event(command)
    }

    /// Build a structured unavailable result for hosts without a renderer.
    pub fn renderer_unavailable(
        reason: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> ApplicationHostCommandResult {
        ApplicationHostCommandResult::unavailable(reason.into(), trace)
    }
}

/// Validate app/session/trace scope required by Route C.
fn require_scope(app_id: &str, session_id: &str, trace: Option<&TraceContext>) -> GenUiResult<()> {
    if app_id.trim().is_empty() {
        return Err(UiRenderError::MissingApplicationId);
    }
    if session_id.trim().is_empty() {
        return Err(UiRenderError::MissingSessionId);
    }
    if trace.is_none() {
        return Err(UiRenderError::MissingTraceContext);
    }
    Ok(())
}

/// Reject obvious executable script payloads in component props.
///
/// This is intentionally conservative.  GenUI v0 is declarative, so script-like
/// keys are invalid even if a future renderer could theoretically ignore them.
fn reject_script_like_payload(value: &serde_json::Value) -> GenUiResult<()> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let lowered = key.to_ascii_lowercase();
                if lowered.contains("script") || lowered.starts_with("on") {
                    return Err(UiRenderError::UnsafePayload(key.clone()));
                }
                reject_script_like_payload(child)?;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                reject_script_like_payload(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Convenience helper for small tests and fixture surfaces.
pub fn text_component(id: impl Into<String>, text: impl Into<String>) -> UiComponent {
    UiComponent::new(id, UiComponentKind::Text, json!({ "text": text.into() }))
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        UiAction, UiComponentTree, UiEvent, UiEventCommand, UiRenderSurface, UiTraceMarker,
    };

    use super::*;

    fn trace() -> TraceContext {
        let mut trace = TraceContext::new("trace-genui");
        trace.session_id = Some("session-genui".into());
        trace
    }

    fn intent() -> UiIntent {
        let trace = trace();
        let surface = UiRenderSurface::new("main");
        UiIntent {
            app_id: "app-genui".into(),
            session_id: "session-genui".into(),
            surface_id: surface.clone(),
            tree: UiComponentTree {
                root: UiComponent::new("root", UiComponentKind::Card, json!({"title": "Panel"}))
                    .with_child(text_component("text", "Ready")),
                trace_markers: vec![UiTraceMarker::new(
                    "app-genui",
                    "session-genui",
                    &trace,
                    surface,
                    Some("root".into()),
                )],
                metadata: Default::default(),
            },
            permission_prompts: Vec::new(),
            approval_prompts: Vec::new(),
            trace: Some(trace),
            metadata: Default::default(),
        }
    }

    #[test]
    fn genui_runtime_builds_traced_render_command() {
        let runtime = GenUiRuntime::new();
        let trace = trace();
        let command = runtime.render_command(intent(), trace).unwrap();

        assert_eq!(command.import, ApplicationImport::UiRender);
        assert!(command.trace.is_some());
    }

    #[test]
    fn genui_validator_rejects_missing_trace() {
        let mut intent = intent();
        intent.trace = None;

        let error = UiIntentValidator
            .validate_intent(&intent)
            .expect_err("missing trace must reject");

        assert_eq!(error, UiRenderError::MissingTraceContext);
    }

    #[test]
    fn genui_validator_rejects_unsupported_components() {
        let mut intent = intent();
        intent.tree.root.children.push(UiComponent::new(
            "future",
            UiComponentKind::Custom("future_panel".into()),
            json!({}),
        ));

        let error = UiIntentValidator
            .validate_intent(&intent)
            .expect_err("unsupported component must reject");

        assert!(matches!(error, UiRenderError::UnsupportedComponent(_)));
    }

    #[test]
    fn genui_validator_rejects_script_like_payloads() {
        let mut intent = intent();
        intent.tree.root.children.push(UiComponent::new(
            "unsafe",
            UiComponentKind::Button,
            json!({"onClick": "alert(1)"}),
        ));

        let error = UiIntentValidator
            .validate_intent(&intent)
            .expect_err("script-like keys must reject");

        assert!(matches!(error, UiRenderError::UnsafePayload(_)));
    }

    #[test]
    fn genui_runtime_validates_event_scope() {
        let command = UiEventCommand::new(UiEvent {
            event_id: "event-1".into(),
            app_id: "app-genui".into(),
            session_id: "session-genui".into(),
            surface_id: UiRenderSurface::new("main"),
            component_id: "button".into(),
            action: UiAction {
                name: "submit".into(),
                label: "Submit".into(),
                payload: json!({}),
                metadata: Default::default(),
            },
            payload: json!({}),
            trace: Some(trace()),
            metadata: Default::default(),
        });

        GenUiRuntime::new().validate_event(&command).unwrap();
    }
}
