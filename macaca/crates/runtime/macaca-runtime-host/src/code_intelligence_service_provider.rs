//! Runtime-host provider for `service.code_intelligence`.
//!
//! The provider is a small Adapter over local repository scanning.  Future
//! GitNexus, LSP, static-analysis, or remote-index providers can replace the
//! `CodeIntelligenceProvider` Strategy without changing SDK, shell, or
//! application contracts.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::code_intelligence::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tracing::{info, warn};

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

const SCAN_FILE_LIMIT: usize = 1024;

#[async_trait]
pub trait CodeIntelligenceProvider: Send + Sync {
    async fn search(&self, request: CodeSearchRequest) -> ServiceResult<CodeIntelligenceResponse>;
    async fn symbol_context(
        &self,
        request: SymbolContextRequest,
    ) -> ServiceResult<CodeIntelligenceResponse>;
    async fn references(
        &self,
        request: FileReferenceRequest,
    ) -> ServiceResult<CodeIntelligenceResponse>;
    async fn diagnostics(
        &self,
        request: AnalyzerDiagnosticsRequest,
    ) -> ServiceResult<CodeIntelligenceResponse>;
}

#[derive(Debug, Default)]
pub struct LocalCodeIntelligenceProvider;

pub struct CodeIntelligenceSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn CodeIntelligenceProvider>>,
    unavailable_reason: Option<String>,
}

pub async fn bootstrap_local_code_intelligence_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(CodeIntelligenceSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(service_id = %service_id, trace_id = %trace.trace_id, "code intelligence service registering local provider");
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
    info!(service_id = %service_id, trace_id = %trace.trace_id, "code intelligence service local provider started");
    Ok(service_id)
}

impl CodeIntelligenceSystemServiceProvider {
    pub fn local() -> Self {
        Self::new(Arc::new(LocalCodeIntelligenceProvider))
    }

    pub fn mock() -> Self {
        Self::local()
    }

    pub fn new(provider: Arc<dyn CodeIntelligenceProvider>) -> Self {
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

    fn provider(&self) -> ServiceResult<Arc<dyn CodeIntelligenceProvider>> {
        self.provider.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "code intelligence provider is not configured".into()),
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
        response: CodeIntelligenceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary: None,
            audit_ref: Some(format!("code-intelligence:{}", trace.trace_id)),
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
            CodeIntelligenceResponse::Diagnostics {
                diagnostics: Vec::new(),
                provider_health: health(false, self.unavailable_reason.clone()),
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "code intelligence provider is not configured".into()),
            },
        )
    }
}

#[async_trait]
impl SystemService for CodeIntelligenceSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, configured = self.provider.is_some(), "code intelligence service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "code intelligence service command accepted");
        if self.provider.is_none() {
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, "code intelligence unavailable provider returned structured unavailable");
            return self.unavailable_result(trace).await;
        }
        let response = match command.name.as_str() {
            SEARCH_COMMAND => self.provider()?.search(decode(command.payload)?).await?,
            SYMBOL_CONTEXT_COMMAND => {
                self.provider()?
                    .symbol_context(decode(command.payload)?)
                    .await?
            }
            REFERENCES_FIND_COMMAND => {
                self.provider()?
                    .references(decode(command.payload)?)
                    .await?
            }
            DIAGNOSTICS_COMMAND => {
                self.provider()?
                    .diagnostics(decode(command.payload)?)
                    .await?
            }
            "service.snapshot" => CodeIntelligenceResponse::Diagnostics {
                diagnostics: Vec::new(),
                provider_health: health(true, None),
            },
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        Self::result(response, trace, WorkbenchCommandStatus::Completed)
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "code intelligence service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "code intelligence service cleanup completed");
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
impl CodeIntelligenceProvider for LocalCodeIntelligenceProvider {
    async fn search(&self, request: CodeSearchRequest) -> ServiceResult<CodeIntelligenceResponse> {
        if request.query.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "code.search requires non-empty query".into(),
            ));
        }
        let files = candidate_files(&request.workspace_root, &request.include_globs)?;
        let mut results = Vec::new();
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (index, line) in text.lines().enumerate() {
                if line.contains(&request.query) {
                    results.push(result_for(
                        &request.workspace_root,
                        &path,
                        index,
                        line,
                        request.snippet_budget_bytes,
                    )?);
                    if results.len() >= request.max_results.max(1) {
                        return Ok(CodeIntelligenceResponse::Search {
                            results,
                            truncated: true,
                            provider_health: health(true, None),
                        });
                    }
                }
            }
        }
        Ok(CodeIntelligenceResponse::Search {
            results,
            truncated: false,
            provider_health: health(true, None),
        })
    }

    async fn symbol_context(
        &self,
        request: SymbolContextRequest,
    ) -> ServiceResult<CodeIntelligenceResponse> {
        let search = CodeSearchRequest {
            workspace_root: request.workspace_root.clone(),
            query: request.symbol.clone(),
            include_globs: Vec::new(),
            max_results: request.max_references.max(1),
            snippet_budget_bytes: request.snippet_budget_bytes,
        };
        let CodeIntelligenceResponse::Search { results, .. } = self.search(search).await? else {
            unreachable!("local search always returns search response");
        };
        Ok(CodeIntelligenceResponse::SymbolContext(SymbolContext {
            symbol: request.symbol,
            definitions: results.iter().take(1).cloned().collect(),
            references: results,
            provider_health: health(true, None),
        }))
    }

    async fn references(
        &self,
        request: FileReferenceRequest,
    ) -> ServiceResult<CodeIntelligenceResponse> {
        let search = CodeSearchRequest {
            workspace_root: request.workspace_root,
            query: request.path.clone(),
            include_globs: Vec::new(),
            max_results: request.max_results.max(1),
            snippet_budget_bytes: 240,
        };
        let CodeIntelligenceResponse::Search {
            results, truncated, ..
        } = self.search(search).await?
        else {
            unreachable!("local search always returns search response");
        };
        Ok(CodeIntelligenceResponse::References {
            path: request.path,
            results,
            truncated,
        })
    }

    async fn diagnostics(
        &self,
        request: AnalyzerDiagnosticsRequest,
    ) -> ServiceResult<CodeIntelligenceResponse> {
        let root = workspace_root(&request.workspace_root)?;
        let mut diagnostics = Vec::new();
        for rel in request
            .changed_paths
            .iter()
            .take(request.max_diagnostics.max(1))
        {
            reject_unsafe_relative_path(rel)?;
            let path = root.join(rel);
            if !path.exists() {
                diagnostics.push(AnalyzerDiagnostic {
                    severity: AnalyzerSeverity::Warning,
                    location: CodeLocation {
                        path: rel.clone(),
                        line: 0,
                        column: 0,
                    },
                    message: summary("path is not present in analyzer workspace", 240),
                    analyzer_ref: request
                        .analyzer_ref
                        .clone()
                        .unwrap_or_else(|| "local.path-presence".into()),
                });
            }
        }
        Ok(CodeIntelligenceResponse::Diagnostics {
            diagnostics,
            provider_health: health(true, None),
        })
    }
}

fn candidate_files(workspace: &str, globs: &[String]) -> ServiceResult<Vec<PathBuf>> {
    let root = workspace_root(workspace)?;
    let mut out = Vec::new();
    collect_files(&root, &root, globs, &mut out)?;
    Ok(out)
}

fn collect_files(
    root: &Path,
    dir: &Path,
    globs: &[String],
    out: &mut Vec<PathBuf>,
) -> ServiceResult<()> {
    if out.len() >= SCAN_FILE_LIMIT {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        if path.is_dir() {
            if path.file_name().and_then(|item| item.to_str()) != Some(".git") {
                collect_files(root, &path, globs, out)?;
            }
        } else if should_include(root, &path, globs) {
            out.push(path);
        }
    }
    Ok(())
}

fn should_include(root: &Path, path: &Path, globs: &[String]) -> bool {
    if globs.is_empty() {
        return true;
    }
    let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
    globs.iter().any(|item| rel.contains(item))
}

fn result_for(
    workspace: &str,
    path: &Path,
    index: usize,
    line: &str,
    budget: usize,
) -> ServiceResult<CodeSearchResult> {
    Ok(CodeSearchResult {
        location: CodeLocation {
            path: public_path(workspace, path)?,
            line: (index + 1) as u32,
            column: 1,
        },
        snippet: summary(line, budget),
        score: 100,
    })
}

fn summary(text: &str, budget: usize) -> BoundedSummary {
    let budget = budget.max(1);
    let truncated = text.len() > budget;
    BoundedSummary {
        text: text.chars().take(budget).collect(),
        truncated,
        original_byte_len: Some(text.len() as u64),
        artifact_ref: None,
    }
}

fn health(healthy: bool, reason: Option<String>) -> ProviderHealthDiagnostic {
    ProviderHealthDiagnostic {
        provider_ref: "local.code-intelligence".into(),
        healthy,
        generated_at: chrono::Utc::now(),
        reason,
    }
}

fn workspace_root(workspace: &str) -> ServiceResult<PathBuf> {
    Path::new(workspace).canonicalize().map_err(io_error)
}

fn public_path(workspace: &str, path: &Path) -> ServiceResult<String> {
    Ok(path
        .strip_prefix(workspace_root(workspace)?)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string())
}

fn reject_unsafe_relative_path(path: &str) -> ServiceResult<()> {
    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ServiceError::DisabledByPolicy(
                    "absolute paths and traversal are denied".into(),
                ));
            }
        }
    }
    Ok(())
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
