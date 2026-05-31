//! Provider strategy registry for application execution.
//!
//! The registry applies the Strategy pattern at the service boundary.  Concrete
//! hosted runtimes, external application backends, and remote agents all expose
//! the same provider descriptor and receive the same typed commands.  Selection
//! is based on declared protocol metadata, health, capabilities, and caller
//! preference; it never branches on application names or product workflows.

use std::collections::BTreeSet;
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

const PROFILE_KEY: &str = "application_execution.profile";
const TENANT_KEY: &str = "application_execution.tenant";
const ALLOWED_PROVIDER_IDS_KEY: &str = "application_execution.policy.allowed_provider_ids";
const DENIED_PROVIDER_IDS_KEY: &str = "application_execution.policy.denied_provider_ids";

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

    /// Return current health for provider selection, diagnostics, and routing.
    ///
    /// The default implementation reads the descriptor because simple providers
    /// may publish static health.  Dynamic providers should override this method
    /// and compute health from their transport, lease table, worker pool, or
    /// runtime state without exposing raw provider payloads.
    async fn health(&self) -> Result<ApplicationExecutionProviderHealth, ServiceError> {
        Ok(self.describe().health_state)
    }

    /// Resume provider work from a sanitized snapshot or checkpoint.
    ///
    /// Provider implementations that support checkpointing should override this
    /// method and map the memento back into their generic execution loop.  The
    /// default returns a structured unsupported result rather than pretending
    /// resume happened, which keeps replay and recovery audit trails honest.
    async fn resume(
        &self,
        snapshot: ApplicationExecutionSnapshot,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        let scope = snapshot.scope;
        let provider_kind = snapshot.provider_kind;
        Ok(StartApplicationExecutionResult {
            status: ApplicationExecutionCommandStatus::Unsupported,
            session_id: Some(scope.session_id.clone()),
            run_id: Some(scope.run_id.clone()),
            provider_id: snapshot.provider_id,
            provider_kind,
            event_cursor: snapshot.latest_event_cursor,
            control_ref: None,
            workspace_ref: None,
            error: Some(ApplicationExecutionError {
                code: ApplicationExecutionCommandStatus::Unsupported,
                layer: "service.application_execution.provider".into(),
                operation: "resume".into(),
                application_id: Some(scope.application_id),
                session_id: Some(scope.session_id),
                run_id: Some(scope.run_id),
                provider_id: None,
                provider_kind: Some(provider_kind),
                trace_id: None,
                reason: "application execution provider does not support resume".into(),
                retryable: false,
            }),
        })
    }

    /// Gracefully stop provider-owned background resources.
    ///
    /// Providers that own worker tasks, callback leases, sockets, or transport
    /// clients should override this method and release them through their
    /// adapter boundary.  The no-op default is safe for stateless test and
    /// unavailable providers.
    async fn shutdown(&self) -> Result<(), ServiceError> {
        Ok(())
    }
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

/// Policy-gated provider filters supplied by service admission code.
///
/// The registry keeps policy as data instead of calling concrete policy
/// engines.  That keeps selection deterministic, testable, and reusable for
/// hosted, external-backend, and remote-agent providers while the ServiceRuntime
/// decorator remains the owner of policy evaluation before side effects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationExecutionProviderSelectionPolicy {
    pub allowed_provider_ids: BTreeSet<String>,
    pub denied_provider_ids: BTreeSet<String>,
    pub allowed_provider_kinds: BTreeSet<ApplicationExecutionProviderKind>,
    pub denied_provider_kinds: BTreeSet<ApplicationExecutionProviderKind>,
}

/// Complete provider selection input for the Strategy registry.
///
/// `execution_profile` is the manifest-level execution shape normalized by the
/// application framework or service adapter before registry selection.  It is a
/// generic protocol label, not an application name.  `tenant_id` is optional
/// because local developer and single-tenant deployments can still use the same
/// provider strategy path without inventing a fake tenant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationExecutionProviderSelectionCriteria {
    pub provider_preference: Option<ApplicationExecutionProviderPreference>,
    pub required_capabilities: Vec<CapabilityId>,
    pub execution_profile: Option<String>,
    pub tenant_id: Option<String>,
    pub policy: ApplicationExecutionProviderSelectionPolicy,
}

impl ApplicationExecutionProviderSelectionCriteria {
    /// Build provider-selection criteria from a typed start command.
    ///
    /// The application framework or caller may place generic execution-profile
    /// and provider-policy hints in `policy_context`.  This helper is the only
    /// place where the registry interprets those keys, keeping the service
    /// provider thin and preventing shells from owning provider routing
    /// semantics.  Unknown keys are ignored so future policy engines can add
    /// context without breaking older runtimes.
    pub fn from_start_command(command: &StartApplicationExecutionCommand) -> Self {
        Self {
            provider_preference: command.provider_preference.clone(),
            required_capabilities: command.requested_capabilities.clone(),
            execution_profile: command.policy_context.get(PROFILE_KEY).cloned(),
            tenant_id: command.tenant_id.clone(),
            policy: ApplicationExecutionProviderSelectionPolicy {
                allowed_provider_ids: parse_policy_set(
                    command.policy_context.get(ALLOWED_PROVIDER_IDS_KEY),
                ),
                denied_provider_ids: parse_policy_set(
                    command.policy_context.get(DENIED_PROVIDER_IDS_KEY),
                ),
                allowed_provider_kinds: BTreeSet::new(),
                denied_provider_kinds: BTreeSet::new(),
            },
        }
    }
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
    pub async fn select(
        &self,
        preference: Option<&ApplicationExecutionProviderPreference>,
        required_capabilities: &[CapabilityId],
    ) -> Result<ApplicationExecutionProviderSelection, ApplicationExecutionError> {
        self.select_with_criteria(ApplicationExecutionProviderSelectionCriteria {
            provider_preference: preference.cloned(),
            required_capabilities: required_capabilities.to_vec(),
            execution_profile: None,
            tenant_id: None,
            policy: ApplicationExecutionProviderSelectionPolicy::default(),
        })
        .await
    }

    /// Select one provider using the full provider Strategy criteria.
    ///
    /// Selection is a side-effect-free Specification pass except for querying
    /// each provider's bounded health method.  The ordered `considered` records
    /// are retained so service logs and audit records can explain assignment
    /// without leaking manifests, provider payloads, or tenant-private policy
    /// inputs.
    pub async fn select_with_criteria(
        &self,
        criteria: ApplicationExecutionProviderSelectionCriteria,
    ) -> Result<ApplicationExecutionProviderSelection, ApplicationExecutionError> {
        let mut considered = Vec::new();
        for provider in &self.providers {
            let mut descriptor = provider.describe();
            let health = provider.health().await.unwrap_or_else(|error| {
                ApplicationExecutionProviderHealth::Unavailable {
                    reason: error.to_string(),
                }
            });
            descriptor.health_state = health.clone();
            let rejection = provider_rejection_reason(&descriptor, &criteria, &health);
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
    criteria: &ApplicationExecutionProviderSelectionCriteria,
    health: &ApplicationExecutionProviderHealth,
) -> Option<String> {
    if let Some(preference) = &criteria.provider_preference {
        if preference.provider_kind != descriptor.provider_kind {
            return Some("provider kind preference mismatch".into());
        }
        if let Some(provider_id) = &preference.provider_id {
            if provider_id != &descriptor.provider_id {
                return Some("provider id preference mismatch".into());
            }
        }
    }
    if !criteria.policy.allowed_provider_ids.is_empty()
        && !criteria
            .policy
            .allowed_provider_ids
            .contains(&descriptor.provider_id)
    {
        return Some("provider policy denied".into());
    }
    if criteria
        .policy
        .denied_provider_ids
        .contains(&descriptor.provider_id)
    {
        return Some("provider policy denied".into());
    }
    if !criteria.policy.allowed_provider_kinds.is_empty()
        && !criteria
            .policy
            .allowed_provider_kinds
            .contains(&descriptor.provider_kind)
    {
        return Some("provider policy denied".into());
    }
    if criteria
        .policy
        .denied_provider_kinds
        .contains(&descriptor.provider_kind)
    {
        return Some("provider policy denied".into());
    }
    if let Some(profile) = &criteria.execution_profile {
        if let Some(declared) = descriptor.resource_profile.get(PROFILE_KEY) {
            if declared != profile {
                return Some("provider execution profile mismatch".into());
            }
        }
    }
    if let Some(tenant_id) = &criteria.tenant_id {
        if let Some(declared) = descriptor.resource_profile.get(TENANT_KEY) {
            if declared != tenant_id {
                return Some("provider tenant constraint mismatch".into());
            }
        }
    }
    if matches!(
        health,
        ApplicationExecutionProviderHealth::Unavailable { .. }
    ) {
        return Some("provider health unavailable".into());
    }
    for required in &criteria.required_capabilities {
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

/// Parse a comma-separated policy label set from bounded command metadata.
///
/// Provider ids are protocol identities, not application or business names.
/// Empty labels are ignored so callers can pass operator-edited policy strings
/// without creating accidental match entries.
fn parse_policy_set(value: Option<&String>) -> BTreeSet<String> {
    value
        .map(|value| {
            value
                .split(',')
                .filter_map(|label| {
                    let label = label.trim();
                    (!label.is_empty()).then(|| label.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}
