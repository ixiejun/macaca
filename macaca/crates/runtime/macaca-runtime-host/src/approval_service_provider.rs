//! Runtime-host provider for `service.approval`.
//!
//! The provider owns approval lifecycle state, reviewer-policy resolution,
//! shell-visible events, sanitized audit mementos, and structured unavailable
//! behavior.  Reviewer selection is an injected Strategy so tenant, managed,
//! remote, or guardian-backed policies can replace the built-in local policy
//! without changing SDK, shell, kernel, or application code.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::approval::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

const DEFAULT_APPROVAL_PROFILE: &str = "approval.manual.guardian";
const DEFAULT_REVIEWER_CLASS: &str = "manual_guardian";
pub(crate) const LIST_LIMIT_DEFAULT: usize = 100;
pub(crate) const LIST_LIMIT_MAX: usize = 500;

/// Strategy interface for resolving reviewer policy.
#[async_trait]
pub trait ApprovalReviewerPolicy: Send + Sync {
    /// Explain which reviewer class and profile apply to one side-effect class.
    async fn explain(
        &self,
        request: ApprovalPolicyExplain,
    ) -> ServiceResult<ApprovalPolicyExplanation>;
}

/// Built-in local reviewer policy used by the local provider and tests.
///
/// It deliberately returns generic reviewer classes rather than people,
/// application names, or product workflows.  Future providers can use tenant
/// policy, managed requirements, risk scoring, or remote guardians behind the
/// same Strategy interface.
#[derive(Debug, Default)]
pub struct LocalApprovalReviewerPolicy;

#[async_trait]
impl ApprovalReviewerPolicy for LocalApprovalReviewerPolicy {
    async fn explain(
        &self,
        request: ApprovalPolicyExplain,
    ) -> ServiceResult<ApprovalPolicyExplanation> {
        let profile = request
            .approval_profile_ref
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_APPROVAL_PROFILE.into());
        Ok(ApprovalPolicyExplanation {
            required: true,
            approval_profile_ref: profile,
            reviewer_class: DEFAULT_REVIEWER_CLASS.into(),
            reason_code: format!("approval_required.{:?}", request.side_effect_class)
                .to_ascii_lowercase(),
            managed_by_service: true,
        })
    }
}

/// ServiceRuntime provider for approval commands.
pub struct ApprovalSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) reviewer_policy: Option<Arc<dyn ApprovalReviewerPolicy>>,
    pub(crate) records: RwLock<HashMap<String, ApprovalRecord>>,
    pub(crate) audits: RwLock<HashMap<String, ApprovalAuditRecord>>,
    pub(crate) notify_tx: broadcast::Sender<ApprovalEvent>,
    pub(crate) unavailable_reason: Option<String>,
}

/// Register and start the built-in local `service.approval` provider.
///
/// Runtime-host is the approved Abstract Factory composition root.  This keeps
/// provider construction out of the kernel, SDK, shells, and applications while
/// making the service discoverable through the normal service runtime.
pub async fn bootstrap_local_approval_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(ApprovalSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "approval service registering local provider"
    );
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace.clone())
        .await
        .map_err(runtime_error)?;
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "approval service local provider started"
    );
    Ok(service_id)
}

impl ApprovalSystemServiceProvider {
    /// Build the built-in local provider with in-memory state.
    pub fn local() -> Self {
        Self::new(Arc::new(LocalApprovalReviewerPolicy))
    }

    /// Build a mock/local provider from an injected reviewer policy Strategy.
    pub fn new(reviewer_policy: Arc<dyn ApprovalReviewerPolicy>) -> Self {
        let (notify_tx, _) = broadcast::channel(1024);
        Self {
            descriptor: descriptor().base,
            reviewer_policy: Some(reviewer_policy),
            records: RwLock::new(HashMap::new()),
            audits: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason: None,
        }
    }

    /// Build a Null Object provider for hosts where approvals are disabled.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let (notify_tx, _) = broadcast::channel(16);
        Self {
            descriptor: descriptor().base,
            reviewer_policy: None,
            records: RwLock::new(HashMap::new()),
            audits: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason: Some(reason.into()),
        }
    }

    /// Subscribe to bounded approval lifecycle events for shells and gateways.
    pub fn subscribe(&self) -> broadcast::Receiver<ApprovalEvent> {
        self.notify_tx.subscribe()
    }

    pub(crate) fn reviewer_policy(&self) -> ServiceResult<Arc<dyn ApprovalReviewerPolicy>> {
        self.reviewer_policy.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "approval provider is not configured".into()),
            )
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn result(
        response: ApprovalServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        summary: Option<BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary,
            audit_ref: Some(format!("approval:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

#[async_trait]
impl SystemService for ApprovalSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.reviewer_policy.is_some(),
            "approval service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "approval service command accepted"
        );
        if self.reviewer_policy.is_none() {
            warn!(
                service_id = %self.descriptor.id,
                trace_id = %trace.trace_id,
                "approval service unavailable provider returned structured unavailable"
            );
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            REQUEST_CREATE_COMMAND => {
                self.create(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            REQUEST_LIST_COMMAND => {
                self.list(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            REQUEST_READ_COMMAND => {
                self.read(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            REQUEST_RESOLVE_COMMAND => {
                self.resolve(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            REQUEST_CANCEL_COMMAND => {
                self.cancel(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            REQUEST_EXPIRE_COMMAND => {
                self.expire(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            POLICY_EXPLAIN_COMMAND => {
                self.policy_explain(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            DECISION_AUDIT_COMMAND => {
                self.decision_audit(
                    crate::approval_service_commands::decode(command.payload)?,
                    trace,
                )
                .await
            }
            "service.snapshot" => {
                let count = self.records.read().await.len();
                Self::result(
                    ApprovalServiceResponse::Listed {
                        records: Vec::new(),
                        truncated: count > 0,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    Some(BoundedSummary {
                        text: format!("approval records tracked: {count}"),
                        truncated: false,
                        original_byte_len: None,
                        artifact_ref: None,
                    }),
                )
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "approval service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.records.write().await.clear();
        self.audits.write().await.clear();
        info!(service_id = %self.descriptor.id, "approval service cleanup cleared local state");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
        } else {
            Ok(ServiceHealth::Healthy)
        }
    }
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
