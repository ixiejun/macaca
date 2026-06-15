//! Host-owned bootstrap for Workbench-family system services.
//!
//! These services are provider infrastructure, not HTTP presentation behavior.
//! Keeping the registration sequence in host composition lets shells call one
//! typed bootstrap helper while runtime-host remains the concrete provider
//! owner for file, process, sandbox, approval, diagnostics, and related
//! workbench capabilities.

use std::sync::Arc;

use macaca_proto::MacacaResult;
use tracing::info;

use crate::service_runtime::ServiceRuntime;

/// Summary emitted after Workbench-family service bootstrap completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchServiceBootstrapReport {
    /// Number of service bootstrap operations requested by this helper.
    pub requested_services: usize,
    /// Startup trace label supplied by the composition root.
    pub trace_label: String,
}

impl WorkbenchServiceBootstrapReport {
    /// Create a report for the known Workbench-family service set.
    fn new(trace_label: &str) -> Self {
        Self {
            requested_services: 15,
            trace_label: trace_label.to_string(),
        }
    }
}

/// Register and start all host-owned Workbench-family services.
///
/// Design pattern: **Facade + Abstract Factory**. The shell supplies the shared
/// [`ServiceRuntime`] and an audit-friendly trace label. Runtime-host provider
/// factories still construct each concrete service; this helper only owns the
/// stable composition sequence so presentation shells do not encode provider
/// lifecycle semantics.
pub async fn bootstrap_workbench_family_services(
    service_runtime: Arc<ServiceRuntime>,
    trace_label: &str,
) -> MacacaResult<WorkbenchServiceBootstrapReport> {
    info!(
        trace_label,
        "host composition bootstrap starting workbench-family services"
    );

    crate::service_bootstrap::bootstrap_local_app_protocol_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-app-protocol-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_process_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-process-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_sandbox_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-sandbox-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_diagnostics_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-diagnostics-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_unavailable_realtime_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-realtime-service"),
        "realtime provider is not configured",
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_remote_environment_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-remote-environment-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_file_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-file-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_approval_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-approval-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_hook_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-hook-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_config_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-config-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_plugin_marketplace_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-plugin-marketplace-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_code_intelligence_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-code-intelligence-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_git_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-git-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_local_review_service(
        Arc::clone(&service_runtime),
        &format!("{trace_label}-review-service"),
    )
    .await?;
    crate::service_bootstrap::bootstrap_tool_planning_service(
        Arc::clone(&service_runtime),
        Arc::new(crate::service_bootstrap::industrial_tool_planning_service()?),
        &format!("{trace_label}-tool-service"),
    )
    .await?;

    let report = WorkbenchServiceBootstrapReport::new(trace_label);
    info!(
        trace_label = %report.trace_label,
        requested_services = report.requested_services,
        "host composition bootstrap completed workbench-family services"
    );
    Ok(report)
}
