use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{
    ServiceHealth, ToolArtifactRef, ToolEnvironmentArtifactRoot, ToolEnvironmentCleanupCommand,
    ToolEnvironmentProcessHandle, ToolRuntimeEnvironmentDescriptor, ToolRuntimeEnvironmentKind,
    ToolRuntimeEnvironmentScope, ToolRuntimeEnvironmentState, TraceContext,
};

use crate::{
    StaticToolRuntimeEnvironmentProvider, ToolRuntimeEnvironmentService,
    UnavailableToolRuntimeEnvironmentProvider,
};

#[tokio::test]
async fn tool_service_environment_reports_unavailable_provider() {
    let provider = Arc::new(UnavailableToolRuntimeEnvironmentProvider::new(
        "provider.sandbox.unavailable",
        "env.sandbox",
        ToolRuntimeEnvironmentKind::LocalSandbox,
        "sandbox provider is not installed",
    ));
    let service = ToolRuntimeEnvironmentService::new(vec![provider]);

    let result = service
        .health(TraceContext::new("trace-env-unavailable"))
        .await
        .unwrap();

    assert_eq!(result.descriptors.len(), 1);
    assert!(matches!(
        result.descriptors[0].health,
        ServiceHealth::Unavailable { .. }
    ));
    assert_eq!(
        serde_json::to_value(&result.audit_refs[0]).unwrap(),
        "tool.environment.health"
    );
}

#[tokio::test]
async fn tool_service_environment_cleanup_releases_sanitized_handles() {
    let mut descriptor = ToolRuntimeEnvironmentDescriptor::unavailable(
        "env.session",
        "provider.local",
        ToolRuntimeEnvironmentKind::LocalWorkspace,
        "not used",
    );
    descriptor.state = ToolRuntimeEnvironmentState::Ready;
    descriptor.health = ServiceHealth::Healthy;
    descriptor.artifact_roots.push(ToolEnvironmentArtifactRoot {
        root_ref: ToolArtifactRef("artifact.root.session".into()),
        scope: ToolRuntimeEnvironmentScope::Session,
        metadata: BTreeMap::new(),
    });
    descriptor
        .process_handles
        .push(ToolEnvironmentProcessHandle {
            handle_ref: "process.handle.1".into(),
            state: ToolRuntimeEnvironmentState::Busy,
            pid_hint: Some(100),
            metadata: BTreeMap::new(),
        });
    let provider = Arc::new(StaticToolRuntimeEnvironmentProvider::new(descriptor));
    let service = ToolRuntimeEnvironmentService::new(vec![provider]);

    let result = service
        .cleanup(ToolEnvironmentCleanupCommand {
            trace: TraceContext::new("trace-env-cleanup"),
            environment_id: "env.session".into(),
            reason_code: "session_finished".into(),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();

    assert_eq!(result.status, "released");
    assert_eq!(result.released_process_handles, vec!["process.handle.1"]);
    assert_eq!(result.released_artifact_roots[0].0, "artifact.root.session");
}

#[tokio::test]
async fn tool_service_environment_cleanup_unknown_target_is_structured_unavailable() {
    let service = ToolRuntimeEnvironmentService::new(Vec::new());

    let result = service
        .cleanup(ToolEnvironmentCleanupCommand {
            trace: TraceContext::new("trace-env-cleanup-missing"),
            environment_id: "env.missing".into(),
            reason_code: "session_finished".into(),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();

    assert_eq!(result.status, "unavailable");
    assert_eq!(
        serde_json::to_value(&result.audit_refs[0]).unwrap(),
        "tool.environment.cleanup.not_found"
    );
}
