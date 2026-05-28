//! Runtime-host provider for `service.review`.
//!
//! The provider owns a generic review run state machine and structured finding
//! output.  It uses State for run lifecycle, Observer-friendly audit refs for
//! progress/result calls, and Null Object unavailable behavior when no review
//! provider is configured.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::review::*;
use macaca_proto::{
    CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

#[async_trait]
pub trait ReviewProvider: Send + Sync {
    async fn start(&self, request: ReviewStartRequest) -> ServiceResult<ReviewServiceResponse>;
    async fn progress(&self, request: ReviewRefRequest) -> ServiceResult<ReviewServiceResponse>;
    async fn result(
        &self,
        request: ReviewRefRequest,
        trace: &TraceContext,
    ) -> ServiceResult<ReviewServiceResponse>;
    async fn findings(&self, request: ReviewRefRequest) -> ServiceResult<ReviewServiceResponse>;
    async fn clear(&self) -> ServiceResult<()>;
}

#[derive(Debug, Default)]
pub struct LocalReviewProvider {
    runs: RwLock<HashMap<String, ReviewResult>>,
}

pub struct ReviewSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn ReviewProvider>>,
    unavailable_reason: Option<String>,
}

pub async fn bootstrap_local_review_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(ReviewSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(service_id = %service_id, trace_id = %trace.trace_id, "review service registering local provider");
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
    info!(service_id = %service_id, trace_id = %trace.trace_id, "review service local provider started");
    Ok(service_id)
}

impl ReviewSystemServiceProvider {
    pub fn local() -> Self {
        Self::new(Arc::new(LocalReviewProvider::default()))
    }

    pub fn mock() -> Self {
        Self::local()
    }

    pub fn new(provider: Arc<dyn ReviewProvider>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: Some(provider),
            unavailable_reason: None,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: None,
            unavailable_reason: Some(reason.into()),
        }
    }

    fn provider(&self) -> ServiceResult<Arc<dyn ReviewProvider>> {
        self.provider.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "review provider is not configured".into()),
            )
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: ReviewServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary: None,
            audit_ref: Some(format!("review:{}", trace.trace_id)),
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
        Self::result(
            ReviewServiceResponse::Findings(Vec::new()),
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "review provider is not configured".into()),
            },
        )
    }
}

#[async_trait]
impl SystemService for ReviewSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, configured = self.provider.is_some(), "review service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "review service command accepted");
        if self.provider.is_none() {
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, "review unavailable provider returned structured unavailable");
            return self.unavailable_result(trace).await;
        }
        let response = match command.name.as_str() {
            START_COMMAND => self.provider()?.start(decode(command.payload)?).await?,
            PROGRESS_COMMAND => self.provider()?.progress(decode(command.payload)?).await?,
            RESULT_COMMAND => {
                self.provider()?
                    .result(decode(command.payload)?, &trace)
                    .await?
            }
            FINDINGS_LIST_COMMAND => self.provider()?.findings(decode(command.payload)?).await?,
            "service.snapshot" => ReviewServiceResponse::Findings(Vec::new()),
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        Self::result(response, trace, WorkbenchCommandStatus::Completed)
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "review service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        if let Some(provider) = &self.provider {
            provider.clear().await?;
        }
        info!(service_id = %self.descriptor.id, "review service cleanup cleared review runs");
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

#[async_trait]
impl ReviewProvider for LocalReviewProvider {
    async fn start(&self, request: ReviewStartRequest) -> ServiceResult<ReviewServiceResponse> {
        if request.subject_ref.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "review.start requires non-empty subject_ref".into(),
            ));
        }
        let now = chrono::Utc::now();
        let run = ReviewRun {
            review_ref: format!("review-{}", Uuid::new_v4()),
            subject_ref: request.subject_ref,
            status: ReviewRunStatus::Running,
            progress_percent: 25,
            started_at: now,
            completed_at: None,
            evidence_refs: request.evidence_refs.clone(),
        };
        let findings = request
            .rationale
            .as_ref()
            .filter(|summary| summary.text.to_ascii_lowercase().contains("risk"))
            .map(|summary| {
                vec![ReviewFinding {
                    finding_ref: format!("finding-{}", Uuid::new_v4()),
                    severity: ReviewSeverity::Medium,
                    location: request.changed_paths.first().map(|path| ReviewLocation {
                        path: path.clone(),
                        line: None,
                        column: None,
                    }),
                    title: "review rationale records risk".into(),
                    rationale: summary.clone(),
                    evidence_refs: request.evidence_refs.clone(),
                    trace_refs: Vec::new(),
                }]
            })
            .unwrap_or_default();
        let result = ReviewResult {
            run: run.clone(),
            findings,
            artifact_ref: None,
        };
        self.runs
            .write()
            .await
            .insert(run.review_ref.clone(), result);
        info!(review_ref = %run.review_ref, "review run started");
        Ok(ReviewServiceResponse::Started(run))
    }

    async fn progress(&self, request: ReviewRefRequest) -> ServiceResult<ReviewServiceResponse> {
        let result = self.lookup(&request.review_ref).await?;
        Ok(ReviewServiceResponse::Progress(result.run))
    }

    async fn result(
        &self,
        request: ReviewRefRequest,
        trace: &TraceContext,
    ) -> ServiceResult<ReviewServiceResponse> {
        let mut runs = self.runs.write().await;
        let result = runs
            .get_mut(&request.review_ref)
            .ok_or_else(|| ServiceError::InvalidArgument("unknown review_ref".into()))?;
        result.run.status = ReviewRunStatus::Complete;
        result.run.progress_percent = 100;
        result.run.completed_at.get_or_insert_with(chrono::Utc::now);
        if !result.findings.is_empty() && result.artifact_ref.is_none() {
            result.artifact_ref = Some(macaca_proto::ArtifactRef {
                id: format!("review-artifact-{}", Uuid::new_v4()),
                media_type: "application/json".into(),
                sha256: None,
                byte_len: result.findings.len() as u64,
                redaction_profile: "review.findings.bounded".into(),
            });
        }
        for finding in &mut result.findings {
            if !finding
                .trace_refs
                .iter()
                .any(|item| item == &trace.trace_id)
            {
                finding.trace_refs.push(trace.trace_id.clone());
            }
        }
        info!(review_ref = %result.run.review_ref, finding_count = result.findings.len(), "review result completed");
        Ok(ReviewServiceResponse::Result(result.clone()))
    }

    async fn findings(&self, request: ReviewRefRequest) -> ServiceResult<ReviewServiceResponse> {
        let result = self.lookup(&request.review_ref).await?;
        Ok(ReviewServiceResponse::Findings(result.findings))
    }

    async fn clear(&self) -> ServiceResult<()> {
        self.runs.write().await.clear();
        Ok(())
    }
}

impl LocalReviewProvider {
    async fn lookup(&self, review_ref: &str) -> ServiceResult<ReviewResult> {
        self.runs
            .read()
            .await
            .get(review_ref)
            .cloned()
            .ok_or_else(|| ServiceError::InvalidArgument("unknown review_ref".into()))
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
