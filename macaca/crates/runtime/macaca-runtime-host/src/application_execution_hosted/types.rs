//! Protocol-neutral hosted execution types and Strategy trait seams.
//!
//! These types describe generic lifecycle outcomes and adapter contracts without
//! embedding application names, workflow labels, or provider product identifiers.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionEventType, ApplicationExecutionPayload, ApplicationExecutionSnapshot,
    ServiceError, StartApplicationExecutionCommand,
};

/// Stable provider identifier stamped into hosted execution descriptors and events.
pub(crate) const HOSTED_PROVIDER_ID: &str = "provider.macaca_hosted";

/// Protocol version advertised by the Macaca-hosted execution provider.
pub(crate) const HOSTED_PROTOCOL_VERSION: &str = "application-execution.v1";

/// Transport label recorded in provider descriptors for audit replay.
pub(crate) const HOSTED_TRANSPORT_KIND: &str = "macaca_hosted";

/// Outcome returned by a hosted application adapter after start dispatch.
///
/// The enum models generic lifecycle outcomes only. It deliberately avoids domain
/// labels, workflow names, model names, and provider-specific payloads so the hosted
/// provider can remain reusable across YAML, WASM, GenUI, and headless applications.
#[derive(Debug, Clone, PartialEq)]
pub enum HostedApplicationExecutionOutcome {
    /// The application accepted the run and will continue asynchronously.
    Running {
        checkpoint_ref: Option<String>,
        summary: String,
        signals: Vec<HostedApplicationExecutionSignal>,
    },
    /// The application reached a generic approval wait point.
    WaitingForApproval {
        approval_ref: String,
        checkpoint_ref: Option<String>,
        summary: String,
    },
    /// The application completed synchronously through the hosted adapter.
    Completed { summary: String },
}

/// Provider-neutral signal produced by a hosted runtime adapter.
///
/// A signal is not a raw WASM or application payload. It is the sanitized,
/// protocol-shaped fact that the `MacacaHosted` provider may append to the durable
/// application-execution EventLog.
#[derive(Debug, Clone, PartialEq)]
pub struct HostedApplicationExecutionSignal {
    pub event_type: ApplicationExecutionEventType,
    pub payload: ApplicationExecutionPayload,
    pub idempotency_suffix: String,
}

impl HostedApplicationExecutionSignal {
    /// Build one signal with a bounded payload summary and small JSON metadata.
    pub fn new(
        event_type: ApplicationExecutionEventType,
        summary: impl Into<String>,
        data: Option<serde_json::Value>,
        idempotency_suffix: impl Into<String>,
    ) -> Self {
        let mut payload = ApplicationExecutionPayload::summary(summary);
        payload.data = data;
        Self {
            event_type,
            payload,
            idempotency_suffix: idempotency_suffix.into(),
        }
    }
}

/// Adapter implemented by concrete hosted application runtimes.
///
/// Runtime-host owns this trait because it is a Strategy seam, not a protocol DTO.
/// Implementations must route privileged work through declared capabilities and
/// `ServiceRuntime` rather than directly constructing LLM, file, process, sandbox,
/// driver, skill, MCP, or tool providers.
#[async_trait]
pub trait HostedApplicationExecutionAdapter: Send + Sync {
    /// Start one generic application execution envelope.
    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError>;

    /// Deliver a generic control command to the hosted runtime.
    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError>;

    /// Resume a hosted runtime from a checkpoint or snapshot memento.
    async fn resume(
        &self,
        snapshot: ApplicationExecutionSnapshot,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError>;
}

/// Runtime state for one hosted execution run (Memento cache).
///
/// The authoritative history remains the durable EventLog; this cache supports
/// control routing and diagnostics until replay reconstructs provider state.
#[derive(Debug, Clone)]
pub(crate) struct HostedRunState {
    pub(crate) scope: macaca_proto::ApplicationExecutionScope,
    pub(crate) lifecycle_state: macaca_proto::ApplicationExecutionLifecycleState,
    pub(crate) latest_checkpoint_ref: Option<String>,
    pub(crate) pending_approval_ref: Option<String>,
    pub(crate) latest_event_cursor: Option<String>,
}

/// Map serde failures into adapter errors for hosted ABI bridges.
pub(crate) fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
