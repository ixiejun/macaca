//! Protocol-level GenUI Runtime v0 contracts.
//!
//! GenUI is a controlled declarative UI protocol.  The data structures in this
//! module describe what an application wants to render and which traced events
//! can flow back to the host.  They intentionally do not contain executable
//! JavaScript, frontend component implementations, or application-specific
//! routing logic.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::TraceContext;

/// Stable identifier for a GenUI render surface.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiRenderSurface(String);

impl UiRenderSurface {
    /// Create a surface id after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw surface id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UiRenderSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Controlled component kinds supported by GenUI v0.
///
/// `Custom` preserves future component kinds as data.  Runtime validators and
/// renderers can then produce structured unsupported placeholders instead of
/// panicking or silently dropping UI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiComponentKind {
    Text,
    Markdown,
    Form,
    Button,
    Table,
    Card,
    List,
    ChartPlaceholder,
    TracePanelMount,
    ApprovalPrompt,
    Custom(String),
}

impl UiComponentKind {
    /// Return whether the current host knows how to render this v0 component.
    pub fn is_supported_v0(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for UiComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::Markdown => f.write_str("markdown"),
            Self::Form => f.write_str("form"),
            Self::Button => f.write_str("button"),
            Self::Table => f.write_str("table"),
            Self::Card => f.write_str("card"),
            Self::List => f.write_str("list"),
            Self::ChartPlaceholder => f.write_str("chart_placeholder"),
            Self::TracePanelMount => f.write_str("trace_panel_mount"),
            Self::ApprovalPrompt => f.write_str("approval_prompt"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Trace marker attached to a UI surface or component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiTraceMarker {
    pub app_id: String,
    pub session_id: String,
    pub trace_id: String,
    pub surface_id: UiRenderSurface,
    pub component_id: Option<String>,
    pub emitted_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl UiTraceMarker {
    /// Build a marker from an existing trace context and GenUI scope.
    pub fn new(
        app_id: impl Into<String>,
        session_id: impl Into<String>,
        trace: &TraceContext,
        surface_id: UiRenderSurface,
        component_id: Option<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            session_id: session_id.into(),
            trace_id: trace.trace_id.clone(),
            surface_id,
            component_id,
            emitted_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// UI action advertised by interactive components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiAction {
    pub name: String,
    pub label: String,
    pub payload: serde_json::Value,
    pub metadata: BTreeMap<String, String>,
}

/// Data binding for form fields, table cells, and future reactive surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBinding {
    pub name: String,
    pub target: String,
    pub required: bool,
    pub metadata: BTreeMap<String, String>,
}

/// System-visible permission prompt declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPermissionPrompt {
    pub permission: String,
    pub reason: String,
    pub trace_marker: Option<UiTraceMarker>,
    pub metadata: BTreeMap<String, String>,
}

/// System-owned approval prompt declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiApprovalPrompt {
    pub approval_id: String,
    pub title: String,
    pub body: String,
    pub trace_marker: Option<UiTraceMarker>,
    pub metadata: BTreeMap<String, String>,
}

/// Composite UI node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiComponent {
    pub id: String,
    pub kind: UiComponentKind,
    pub props: serde_json::Value,
    pub children: Vec<UiComponent>,
    pub actions: Vec<UiAction>,
    pub bindings: Vec<UiBinding>,
    pub trace_marker: Option<UiTraceMarker>,
    pub metadata: BTreeMap<String, String>,
}

impl UiComponent {
    /// Build a component with no children or interaction metadata.
    pub fn new(id: impl Into<String>, kind: UiComponentKind, props: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            kind,
            props,
            children: Vec::new(),
            actions: Vec::new(),
            bindings: Vec::new(),
            trace_marker: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Append a child component and return the updated component.
    pub fn with_child(mut self, child: UiComponent) -> Self {
        self.children.push(child);
        self
    }
}

/// Root component tree for one surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiComponentTree {
    pub root: UiComponent,
    pub trace_markers: Vec<UiTraceMarker>,
    pub metadata: BTreeMap<String, String>,
}

/// Application render intent emitted through the ABI host boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiIntent {
    pub app_id: String,
    pub session_id: String,
    pub surface_id: UiRenderSurface,
    pub tree: UiComponentTree,
    pub permission_prompts: Vec<UiPermissionPrompt>,
    pub approval_prompts: Vec<UiApprovalPrompt>,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// User-visible UI event before host dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiEvent {
    pub event_id: String,
    pub app_id: String,
    pub session_id: String,
    pub surface_id: UiRenderSurface,
    pub component_id: String,
    pub action: UiAction,
    pub payload: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Command envelope posted from renderer to host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiEventCommand {
    pub event: UiEvent,
    pub created_at: DateTime<Utc>,
}

impl UiEventCommand {
    /// Wrap a UI event with a host command timestamp.
    pub fn new(event: UiEvent) -> Self {
        Self {
            event,
            created_at: Utc::now(),
        }
    }
}

/// Structured GenUI validation/render errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum UiRenderError {
    #[error("missing trace context")]
    MissingTraceContext,

    #[error("missing application id")]
    MissingApplicationId,

    #[error("missing session id")]
    MissingSessionId,

    #[error("unsupported component kind: {0}")]
    UnsupportedComponent(String),

    #[error("unsafe UI payload: {0}")]
    UnsafePayload(String),

    #[error("renderer unavailable: {0}")]
    RendererUnavailable(String),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn trace() -> TraceContext {
        let mut trace = TraceContext::new("trace-ui");
        trace.session_id = Some("session-ui".into());
        trace
    }

    #[test]
    fn ui_intent_round_trips() {
        let trace = trace();
        let surface = UiRenderSurface::new("main");
        let marker = UiTraceMarker::new(
            "app-ui",
            "session-ui",
            &trace,
            surface.clone(),
            Some("root".into()),
        );
        let root =
            UiComponent::new("root", UiComponentKind::Card, json!({"title": "Status"})).with_child(
                UiComponent::new("text", UiComponentKind::Text, json!({"text": "Ready"})),
            );
        let intent = UiIntent {
            app_id: "app-ui".into(),
            session_id: "session-ui".into(),
            surface_id: surface,
            tree: UiComponentTree {
                root,
                trace_markers: vec![marker.clone()],
                metadata: BTreeMap::new(),
            },
            permission_prompts: vec![UiPermissionPrompt {
                permission: "storage.scoped".into(),
                reason: "Render saved app state".into(),
                trace_marker: Some(marker.clone()),
                metadata: BTreeMap::new(),
            }],
            approval_prompts: vec![UiApprovalPrompt {
                approval_id: "approval-1".into(),
                title: "Approve action".into(),
                body: "Continue?".into(),
                trace_marker: Some(marker),
                metadata: BTreeMap::new(),
            }],
            trace: Some(trace),
            metadata: BTreeMap::new(),
        };

        let decoded: UiIntent =
            serde_json::from_str(&serde_json::to_string(&intent).unwrap()).unwrap();

        assert_eq!(decoded.app_id, "app-ui");
        assert_eq!(decoded.tree.root.children.len(), 1);
        assert_eq!(decoded.permission_prompts.len(), 1);
        assert_eq!(decoded.approval_prompts.len(), 1);
    }

    #[test]
    fn unknown_component_kind_remains_structured() {
        let component = UiComponent::new(
            "future",
            UiComponentKind::Custom("future_panel".into()),
            json!({"safe": true}),
        );
        let decoded: UiComponent =
            serde_json::from_str(&serde_json::to_string(&component).unwrap()).unwrap();

        assert_eq!(decoded.kind.to_string(), "future_panel");
        assert!(!decoded.kind.is_supported_v0());
        assert_eq!(
            UiRenderError::UnsupportedComponent(decoded.kind.to_string()).to_string(),
            "unsupported component kind: future_panel"
        );
    }

    #[test]
    fn ui_event_command_round_trips() {
        let event = UiEvent {
            event_id: "event-1".into(),
            app_id: "app-ui".into(),
            session_id: "session-ui".into(),
            surface_id: UiRenderSurface::new("main"),
            component_id: "button-1".into(),
            action: UiAction {
                name: "submit".into(),
                label: "Submit".into(),
                payload: json!({"intent": "continue"}),
                metadata: BTreeMap::new(),
            },
            payload: json!({"field": "value"}),
            trace: Some(trace()),
            metadata: BTreeMap::new(),
        };
        let command = UiEventCommand::new(event);

        let decoded: UiEventCommand =
            serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();

        assert_eq!(decoded.event.component_id, "button-1");
        assert!(decoded.event.trace.is_some());
    }
}
