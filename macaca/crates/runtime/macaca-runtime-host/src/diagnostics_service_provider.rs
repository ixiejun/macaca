//! Runtime-host provider for `service.diagnostics`.
//!
//! The diagnostics provider is a host-side Facade over sanitized source
//! snapshots.  It uses a Memento-style bundle reference for replayability and a
//! Decorator-style redaction pass before summaries or feedback leave the
//! service boundary.  It intentionally stores only bounded, provider-neutral
//! evidence refs and never raw prompts, file contents, provider payloads, or
//! credentials.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::diagnostics::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

const REDACTED: &str = "<redacted>";

/// Host-owned implementation of diagnostics commands.
pub struct DiagnosticsSystemServiceProvider {
    descriptor: ServiceDescriptor,
    sources: RwLock<HashMap<DiagnosticsSourceKind, DiagnosticsSourceSnapshot>>,
    feedback: RwLock<HashMap<String, DiagnosticsFeedbackReceipt>>,
    unavailable_reason: Option<String>,
}

/// Register and start the built-in local diagnostics provider.
pub async fn bootstrap_local_diagnostics_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(DiagnosticsSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(service_id = %service_id, trace_id = %trace.trace_id, "diagnostics service registering local provider");
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
    info!(service_id = %service_id, trace_id = %trace.trace_id, "diagnostics service local provider started");
    Ok(service_id)
}

impl DiagnosticsSystemServiceProvider {
    /// Build a local provider with generic source families pre-registered.
    pub fn local() -> Self {
        let provider = Self::empty(None);
        provider
    }

    /// Build an in-memory provider suitable for focused tests.
    pub fn mock() -> Self {
        Self::local()
    }

    /// Build a Null Object provider for hosts that disable diagnostics.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::empty(Some(reason.into()))
    }

    fn empty(unavailable_reason: Option<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            sources: RwLock::new(default_sources()),
            feedback: RwLock::new(HashMap::new()),
            unavailable_reason,
        }
    }

    /// Register or replace a sanitized source snapshot.
    pub async fn upsert_source(&self, source: DiagnosticsSourceSnapshot) {
        info!(
            service_id = %self.descriptor.id,
            source = ?source.source,
            source_service_id = %source.service_id,
            "diagnostics source snapshot updated"
        );
        self.sources
            .write()
            .await
            .insert(source.source.clone(), source);
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: DiagnosticsServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary: None,
            audit_ref: Some(format!("diagnostics:{}", trace.trace_id)),
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

    async fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        let reason = self
            .unavailable_reason
            .clone()
            .unwrap_or_else(|| "diagnostics provider is not configured".into());
        warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason = %reason, "diagnostics unavailable provider returned structured unavailable");
        let response = DiagnosticsServiceResponse::HealthSummary(DiagnosticsHealthSummary {
            services: Vec::new(),
            generated_at: chrono::Utc::now(),
        });
        Self::result(
            response,
            trace,
            WorkbenchCommandStatus::Unavailable { reason },
        )
    }

    async fn snapshot(
        &self,
        request: DiagnosticsSnapshotRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let sources = self.filtered_sources(&request.include_sources).await;
        let bundle = DiagnosticsBundle {
            bundle_ref: format!("diagnostics-bundle-{}", Uuid::new_v4()),
            redaction_profile_id: request.redaction_profile.profile_id.clone(),
            sources: redact_sources(sources, &request.redaction_profile),
            artifact_refs: Vec::new(),
            bounded: true,
            created_at: chrono::Utc::now(),
        };
        info!(
            service_id = %self.descriptor.id,
            trace_id = %trace.trace_id,
            bundle_ref = %bundle.bundle_ref,
            source_count = bundle.sources.len(),
            "diagnostics snapshot bundle created"
        );
        Self::result(
            DiagnosticsServiceResponse::Snapshot(bundle),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn feedback_upload(
        &self,
        request: DiagnosticsFeedbackUploadRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let receipt = DiagnosticsFeedbackReceipt {
            feedback_ref: format!("diagnostics-feedback-{}", Uuid::new_v4()),
            uploaded: request.allow_remote_upload,
            summary: redact_summary(request.summary, &request.redaction_profile),
            attachment_refs: request.attachment_refs,
            created_at: chrono::Utc::now(),
        };
        info!(
            service_id = %self.descriptor.id,
            trace_id = %trace.trace_id,
            feedback_ref = %receipt.feedback_ref,
            uploaded = receipt.uploaded,
            "diagnostics feedback receipt created"
        );
        self.feedback
            .write()
            .await
            .insert(receipt.feedback_ref.clone(), receipt.clone());
        Self::result(
            DiagnosticsServiceResponse::FeedbackUploaded(receipt),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn trace_bundle(
        &self,
        request: DiagnosticsTraceBundleRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let summary = BoundedSummary {
            text: format!("{} trace refs bundled", request.trace_refs.len()),
            truncated: false,
            original_byte_len: None,
            artifact_ref: None,
        };
        let source = DiagnosticsSourceSnapshot {
            source: DiagnosticsSourceKind::AppProtocol,
            service_id: "trace.audit".into(),
            health: ServiceHealth::Healthy,
            summary: Some(summary),
            artifact_refs: Vec::new(),
            captured_at: chrono::Utc::now(),
        };
        let bundle = DiagnosticsBundle {
            bundle_ref: format!("diagnostics-trace-bundle-{}", Uuid::new_v4()),
            redaction_profile_id: request.redaction_profile.profile_id.clone(),
            sources: vec![source],
            artifact_refs: Vec::new(),
            bounded: true,
            created_at: chrono::Utc::now(),
        };
        info!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, bundle_ref = %bundle.bundle_ref, "diagnostics trace bundle created");
        Self::result(
            DiagnosticsServiceResponse::TraceBundle(bundle),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn health_summary(
        &self,
        request: DiagnosticsHealthSummaryRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let mut sources = self.filtered_sources(&Vec::new()).await;
        if !request.service_ids.is_empty() {
            sources.retain(|source| request.service_ids.contains(&source.service_id));
        }
        let summary = DiagnosticsHealthSummary {
            services: redact_sources(sources, &request.redaction_profile),
            generated_at: chrono::Utc::now(),
        };
        info!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, service_count = summary.services.len(), "diagnostics health summary created");
        Self::result(
            DiagnosticsServiceResponse::HealthSummary(summary),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn filtered_sources(
        &self,
        requested: &Vec<DiagnosticsSourceKind>,
    ) -> Vec<DiagnosticsSourceSnapshot> {
        let sources = self.sources.read().await;
        sources
            .values()
            .filter(|source| requested.is_empty() || requested.contains(&source.source))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl SystemService for DiagnosticsSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, configured = self.unavailable_reason.is_none(), "diagnostics service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "diagnostics service command accepted");
        if self.unavailable_reason.is_some() {
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            SNAPSHOT_COMMAND => self.snapshot(decode(command.payload)?, trace).await,
            FEEDBACK_UPLOAD_COMMAND => self.feedback_upload(decode(command.payload)?, trace).await,
            TRACE_BUNDLE_COMMAND => self.trace_bundle(decode(command.payload)?, trace).await,
            HEALTH_SUMMARY_COMMAND => self.health_summary(decode(command.payload)?, trace).await,
            "service.snapshot" => {
                self.snapshot(
                    DiagnosticsSnapshotRequest {
                        include_sources: Vec::new(),
                        redaction_profile: DiagnosticsRedactionProfile::default(),
                    },
                    trace,
                )
                .await
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "diagnostics service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.feedback.write().await.clear();
        info!(service_id = %self.descriptor.id, "diagnostics cleanup cleared feedback receipts");
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

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn redact_sources(
    sources: Vec<DiagnosticsSourceSnapshot>,
    profile: &DiagnosticsRedactionProfile,
) -> Vec<DiagnosticsSourceSnapshot> {
    sources
        .into_iter()
        .map(|mut source| {
            source.summary = source
                .summary
                .map(|summary| redact_summary(summary, profile));
            source
        })
        .collect()
}

fn redact_summary(
    mut summary: BoundedSummary,
    profile: &DiagnosticsRedactionProfile,
) -> BoundedSummary {
    let lower = summary.text.to_lowercase();
    if profile
        .secret_markers
        .iter()
        .any(|marker| lower.contains(&marker.to_lowercase()))
    {
        summary.text = REDACTED.into();
        summary.truncated = true;
        summary.artifact_ref = None;
        return summary;
    }
    if summary.text.len() > profile.max_inline_bytes {
        summary.text.truncate(profile.max_inline_bytes);
        summary.truncated = true;
    }
    summary
}

fn default_sources() -> HashMap<DiagnosticsSourceKind, DiagnosticsSourceSnapshot> {
    use DiagnosticsSourceKind::*;
    [
        Interaction,
        File,
        Process,
        Sandbox,
        Approval,
        Hook,
        Config,
        Plugin,
        Mcp,
        Skill,
        Tool,
        Llm,
        Git,
        Review,
        AppProtocol,
    ]
    .into_iter()
    .map(|source| {
        let snapshot = DiagnosticsSourceSnapshot {
            service_id: format!("service.{}", source_name(&source)),
            source: source.clone(),
            health: ServiceHealth::Unavailable {
                reason: "source provider has not reported diagnostics".into(),
            },
            summary: None,
            artifact_refs: Vec::new(),
            captured_at: chrono::Utc::now(),
        };
        (source, snapshot)
    })
    .collect()
}

fn source_name(source: &DiagnosticsSourceKind) -> &'static str {
    match source {
        DiagnosticsSourceKind::Interaction => "interaction",
        DiagnosticsSourceKind::File => "file",
        DiagnosticsSourceKind::Process => "process",
        DiagnosticsSourceKind::Sandbox => "sandbox",
        DiagnosticsSourceKind::Approval => "approval",
        DiagnosticsSourceKind::Hook => "hook",
        DiagnosticsSourceKind::Config => "config",
        DiagnosticsSourceKind::Plugin => "plugin",
        DiagnosticsSourceKind::Mcp => "mcp",
        DiagnosticsSourceKind::Skill => "skill",
        DiagnosticsSourceKind::Tool => "tool",
        DiagnosticsSourceKind::Llm => "llm",
        DiagnosticsSourceKind::Git => "git",
        DiagnosticsSourceKind::Review => "review",
        DiagnosticsSourceKind::AppProtocol => "app_protocol",
    }
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
