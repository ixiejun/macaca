//! Protocol-level Application ABI v0 contracts for Macaca Agent OS.
//!
//! This module is intentionally data-only.  It defines the stable vocabulary
//! that applications, SDK helpers, package loaders, future WASM components,
//! and host runtimes can exchange without importing `macaca-web` or concrete
//! provider crates.  Runtime execution, policy enforcement, service routing,
//! and UI rendering are implemented by higher layers behind these contracts.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{PackageId, PolicyDecision, ServiceCommand, TraceContext};

/// Current Application ABI version introduced by Route C Phase 05.
///
/// The value is string-backed so future hosts can support semantic versions,
/// integer revisions, or compatibility aliases without changing this type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApplicationAbiVersion(String);

impl ApplicationAbiVersion {
    /// Construct an ABI version after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the canonical Phase 05 ABI version.
    pub fn v0() -> Self {
        Self::new("0")
    }

    /// Return the raw ABI version string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ApplicationAbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Exported function that an Application ABI runtime may invoke.
///
/// `Custom` keeps the ABI extensible.  A future application can declare a new
/// export and the current host can preserve it as data even when execution is
/// rejected as unsupported.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationExport {
    Init,
    Start,
    HandleEvent,
    Render,
    Pause,
    Resume,
    Shutdown,
    Upgrade,
    Custom(String),
}

impl ApplicationExport {
    /// Return the canonical ABI export name.
    pub fn as_name(&self) -> &str {
        match self {
            Self::Init => "app:init",
            Self::Start => "app:start",
            Self::HandleEvent => "app:handle_event",
            Self::Render => "app:render",
            Self::Pause => "app:pause",
            Self::Resume => "app:resume",
            Self::Shutdown => "app:shutdown",
            Self::Upgrade => "app:upgrade",
            Self::Custom(value) => value.as_str(),
        }
    }

    /// Return every export required by Application ABI v0.
    pub fn required_v0() -> Vec<Self> {
        vec![
            Self::Init,
            Self::Start,
            Self::HandleEvent,
            Self::Render,
            Self::Pause,
            Self::Resume,
            Self::Shutdown,
            Self::Upgrade,
        ]
    }
}

impl fmt::Display for ApplicationExport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_name())
    }
}

/// Host import that an application may request through the controlled host.
///
/// The import list is an ABI boundary, not an implementation promise.  Missing
/// backing systems must be represented as structured unavailable results.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationImport {
    CapabilityRequest,
    TaskCreateGoal,
    TaskQuery,
    TraceEmit,
    UiRender,
    StorageGet,
    StorageSet,
    PaymentCreateIntent,
    ServiceCall,
    Custom(String),
}

impl ApplicationImport {
    /// Return the canonical ABI import name.
    pub fn as_name(&self) -> &str {
        match self {
            Self::CapabilityRequest => "macaca:capability/request",
            Self::TaskCreateGoal => "macaca:task/create_goal",
            Self::TaskQuery => "macaca:task/query",
            Self::TraceEmit => "macaca:trace/emit",
            Self::UiRender => "macaca:ui/render",
            Self::StorageGet => "macaca:storage/get",
            Self::StorageSet => "macaca:storage/set",
            Self::PaymentCreateIntent => "macaca:payment/create_intent",
            Self::ServiceCall => "macaca:service/call",
            Self::Custom(value) => value.as_str(),
        }
    }

    /// Return every import declared by Application ABI v0.
    pub fn v0() -> Vec<Self> {
        vec![
            Self::CapabilityRequest,
            Self::TaskCreateGoal,
            Self::TaskQuery,
            Self::TraceEmit,
            Self::UiRender,
            Self::StorageGet,
            Self::StorageSet,
            Self::PaymentCreateIntent,
            Self::ServiceCall,
        ]
    }
}

impl fmt::Display for ApplicationImport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_name())
    }
}

/// Provider-neutral declaration of the ABI surface one application exposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationAbiDeclaration {
    pub application_id: String,
    pub package_id: Option<PackageId>,
    pub version: ApplicationAbiVersion,
    pub exports: Vec<ApplicationExport>,
    pub imports: Vec<ApplicationImport>,
    pub permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationAbiDeclaration {
    /// Build a v0 declaration with all required exports and imports.
    pub fn v0(application_id: impl Into<String>) -> Self {
        Self {
            application_id: application_id.into(),
            package_id: None,
            version: ApplicationAbiVersion::v0(),
            exports: ApplicationExport::required_v0(),
            imports: ApplicationImport::v0(),
            permissions: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Attach a package id to the declaration.
    pub fn with_package_id(mut self, package_id: PackageId) -> Self {
        self.package_id = Some(package_id);
        self
    }

    /// Validate that every v0-required export is present.
    pub fn validate_required_exports(&self) -> Result<(), ApplicationAbiError> {
        for required in ApplicationExport::required_v0() {
            if !self.exports.contains(&required) {
                return Err(ApplicationAbiError::MissingExport(required.to_string()));
            }
        }
        Ok(())
    }
}

/// Explicit lifecycle states for ABI applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationLifecycleState {
    Declared,
    Initialized,
    Started,
    Paused,
    Resumed,
    ShuttingDown,
    Stopped,
    Failed { reason: String },
}

/// Auditable lifecycle transition request/result data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationLifecycleTransition {
    pub application_id: String,
    pub from: ApplicationLifecycleState,
    pub to: ApplicationLifecycleState,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Event delivered to an application through `app:handle_event`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// ABI render request for future GenUI and current shell adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationRenderRequest {
    pub application_id: String,
    pub surface: String,
    pub input: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// ABI render result represented as data instead of concrete UI objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationRenderResult {
    pub surface: String,
    pub payload: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Command-style request from an application to the host.
///
/// This is the Command pattern at the ABI boundary: application code supplies
/// an import name, payload, trace context, and metadata; the host decides which
/// service or unavailable result should handle it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationHostCommand {
    pub import: ApplicationImport,
    pub payload: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationHostCommand {
    /// Build a command that already carries trace context.
    pub fn with_trace(
        import: ApplicationImport,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> Self {
        Self {
            import,
            payload,
            trace: Some(trace),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a command without trace context for validation tests.
    pub fn without_trace(import: ApplicationImport, payload: serde_json::Value) -> Self {
        Self {
            import,
            payload,
            trace: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured host command status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationHostCommandStatus {
    Ok,
    Unavailable { reason: String },
    DisabledByPolicy { reason: String },
    Unsupported { reason: String },
    RuntimeUnavailable { reason: String },
    Rejected { reason: String },
}

/// Structured result returned by the host for every ABI import call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationHostCommandResult {
    pub status: ApplicationHostCommandStatus,
    pub output: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub policy: Option<PolicyDecision>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationHostCommandResult {
    /// Build a successful result with JSON output.
    pub fn ok(output: serde_json::Value, trace: Option<TraceContext>) -> Self {
        Self {
            status: ApplicationHostCommandStatus::Ok,
            output,
            trace,
            policy: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Build a structured unavailable result.
    pub fn unavailable(reason: impl Into<String>, trace: Option<TraceContext>) -> Self {
        Self {
            status: ApplicationHostCommandStatus::Unavailable {
                reason: reason.into(),
            },
            output: serde_json::Value::Null,
            trace,
            policy: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Build a structured runtime-unavailable result.
    pub fn runtime_unavailable(reason: impl Into<String>, trace: Option<TraceContext>) -> Self {
        Self {
            status: ApplicationHostCommandStatus::RuntimeUnavailable {
                reason: reason.into(),
            },
            output: serde_json::Value::Null,
            trace,
            policy: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Portable lifecycle checkpoint.
///
/// The checkpoint is a Memento.  It captures serializable application state
/// without exposing process pointers, provider clients, web state, or handles
/// that cannot cross the ABI boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCheckpoint {
    pub application_id: String,
    pub abi_version: ApplicationAbiVersion,
    pub lifecycle_state: ApplicationLifecycleState,
    pub created_at: DateTime<Utc>,
    pub state: serde_json::Value,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationCheckpoint {
    /// Create a checkpoint with the current timestamp.
    pub fn new(
        application_id: impl Into<String>,
        lifecycle_state: ApplicationLifecycleState,
        state: serde_json::Value,
    ) -> Self {
        Self {
            application_id: application_id.into(),
            abi_version: ApplicationAbiVersion::v0(),
            lifecycle_state,
            created_at: Utc::now(),
            state,
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured ABI errors used before or during host dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ApplicationAbiError {
    #[error("missing required export: {0}")]
    MissingExport(String),

    #[error("unsupported export: {0}")]
    UnsupportedExport(String),

    #[error("unsupported import: {0}")]
    UnsupportedImport(String),

    #[error("missing trace context")]
    MissingTraceContext,

    #[error("invalid lifecycle transition: {from:?} -> {to:?}")]
    InvalidLifecycleTransition {
        from: ApplicationLifecycleState,
        to: ApplicationLifecycleState,
    },

    #[error("runtime unavailable: {0}")]
    RuntimeUnavailable(String),

    #[error("host import unavailable: {0}")]
    HostImportUnavailable(String),

    #[error("invalid ABI declaration: {0}")]
    InvalidDeclaration(String),
}

/// Convert a generic service command into an Application ABI host command.
///
/// The helper keeps service-bus calls behind the same host import boundary so
/// applications cannot bypass trace-required service dispatch.
impl From<ServiceCommand> for ApplicationHostCommand {
    fn from(command: ServiceCommand) -> Self {
        Self {
            import: ApplicationImport::ServiceCall,
            payload: serde_json::to_value(command).unwrap_or(serde_json::Value::Null),
            trace: None,
            metadata: BTreeMap::new(),
        }
    }
}
