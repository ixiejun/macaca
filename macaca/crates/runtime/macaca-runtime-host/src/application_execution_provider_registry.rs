//! Provider strategy registry for application execution.
//!
//! The registry applies the Strategy pattern at the service boundary.  Concrete
//! hosted runtimes, external application backends, and remote agents all expose
//! the same provider descriptor and receive the same typed commands.  Selection
//! is based on declared protocol metadata, health, capabilities, and caller
//! preference; it never branches on application names or product workflows.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlResult, ApplicationExecutionError,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderPreference,
    ApplicationExecutionSnapshot, CapabilityId, ServiceError, StartApplicationExecutionCommand,
    StartApplicationExecutionResult,
};
use tracing::{info, warn};

/// Provider adapter implemented by all application execution backends.
#[async_trait]
pub trait ApplicationExecutionProvider: Send + Sync {
    /// Return provider metadata used for admission, routing, health, and audit.
    fn describe(&self) -> ApplicationExecutionProviderDescriptor;

    /// Start one execution run.  The service has already validated trace and
    /// selected this provider; implementations must still avoid raw secret
    /// logging and must report durable facts through the service EventLog path.
    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<StartApplicationExecutionResult, ServiceError>;

    /// Route one control command to the provider's execution loop or transport.
    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError>;

    /// Return a bounded diagnostic snapshot for this provider.
    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError>;
}

/// Diagnostic record for one considered provider during selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationExecutionProviderSelectionRecord {
    pub provider_id: String,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub accepted: bool,
    pub reason: String,
}

/// Selection result exposed to the service for trace/audit logging.
#[derive(Clone)]
pub struct ApplicationExecutionProviderSelection {
    pub provider: Arc<dyn ApplicationExecutionProvider>,
    pub descriptor: ApplicationExecutionProviderDescriptor,
    pub considered: Vec<ApplicationExecutionProviderSelectionRecord>,
}

/// In-memory provider registry used by runtime-host composition roots.
#[derive(Default, Clone)]
pub struct ApplicationExecutionProviderRegistry {
    providers: Vec<Arc<dyn ApplicationExecutionProvider>>,
}

impl ApplicationExecutionProviderRegistry {
    /// Build an empty registry.  Empty selection returns structured unavailable.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider after validating its protocol descriptor.
    pub fn register(
        mut self,
        provider: Arc<dyn ApplicationExecutionProvider>,
    ) -> Result<Self, ServiceError> {
        validate_descriptor(&provider.describe())?;
        self.providers.push(provider);
        Ok(self)
    }

    /// Return descriptors for health and catalog commands.
    pub fn descriptors(&self) -> Vec<ApplicationExecutionProviderDescriptor> {
        self.providers
            .iter()
            .map(|provider| provider.describe())
            .collect()
    }

    /// Select one provider by preference, health, and required capabilities.
    pub fn select(
        &self,
        preference: Option<&ApplicationExecutionProviderPreference>,
        required_capabilities: &[CapabilityId],
    ) -> Result<ApplicationExecutionProviderSelection, ApplicationExecutionError> {
        let mut considered = Vec::new();
        for provider in &self.providers {
            let descriptor = provider.describe();
            let rejection =
                provider_rejection_reason(&descriptor, preference, required_capabilities);
            if let Some(reason) = rejection {
                considered.push(ApplicationExecutionProviderSelectionRecord {
                    provider_id: descriptor.provider_id,
                    provider_kind: descriptor.provider_kind,
                    accepted: false,
                    reason,
                });
                continue;
            }
            considered.push(ApplicationExecutionProviderSelectionRecord {
                provider_id: descriptor.provider_id.clone(),
                provider_kind: descriptor.provider_kind,
                accepted: true,
                reason: "selected".into(),
            });
            info!(
                provider_id = %descriptor.provider_id,
                provider_kind = ?descriptor.provider_kind,
                "application execution provider selected"
            );
            return Ok(ApplicationExecutionProviderSelection {
                provider: Arc::clone(provider),
                descriptor,
                considered,
            });
        }
        warn!(
            considered = considered.len(),
            "application execution provider selection returned unavailable"
        );
        Err(ApplicationExecutionError {
            code: ApplicationExecutionCommandStatus::Unavailable,
            layer: "service.application_execution.provider_registry".into(),
            operation: "select".into(),
            application_id: None,
            session_id: None,
            run_id: None,
            provider_id: None,
            provider_kind: Some(ApplicationExecutionProviderKind::Unavailable),
            trace_id: None,
            reason: "no healthy application execution provider matched request".into(),
            retryable: false,
        })
    }
}

fn validate_descriptor(
    descriptor: &ApplicationExecutionProviderDescriptor,
) -> Result<(), ServiceError> {
    if descriptor.provider_id.trim().is_empty()
        || descriptor.protocol_version.trim().is_empty()
        || descriptor.transport_kind.trim().is_empty()
        || descriptor.supported_events.is_empty()
    {
        return Err(ServiceError::InvalidArgument(
            "application execution provider descriptor is incomplete".into(),
        ));
    }
    Ok(())
}

fn provider_rejection_reason(
    descriptor: &ApplicationExecutionProviderDescriptor,
    preference: Option<&ApplicationExecutionProviderPreference>,
    required_capabilities: &[CapabilityId],
) -> Option<String> {
    if let Some(preference) = preference {
        if preference.provider_kind != descriptor.provider_kind {
            return Some("provider kind preference mismatch".into());
        }
        if let Some(provider_id) = &preference.provider_id {
            if provider_id != &descriptor.provider_id {
                return Some("provider id preference mismatch".into());
            }
        }
    }
    if matches!(
        descriptor.health_state,
        ApplicationExecutionProviderHealth::Unavailable { .. }
    ) {
        return Some("provider health unavailable".into());
    }
    for required in required_capabilities {
        if !descriptor
            .capability_declarations
            .iter()
            .any(|declared| declared == required)
        {
            return Some("required capability not declared".into());
        }
    }
    None
}
