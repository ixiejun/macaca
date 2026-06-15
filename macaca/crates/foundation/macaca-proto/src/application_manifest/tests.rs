//! Contract tests for Application Manifest v1 serde round-trips.
//!
//! Extracted from `application_manifest.rs` so production DTO definitions stay under
//! the OS 500-line constitution while serde shape remains guarded.

use super::*;
use crate::{
    workbench::sandbox::{SandboxRuntimeKind, WORKSPACE_WRITE_PROFILE},
    AbilityImplementationKind, ApplicationAbilityKind, ApplicationWorkbenchCapabilityDeclaration,
    ApplicationWorkbenchEventSubscription, ApplicationWorkbenchManifestDeclaration,
    ApplicationWorkbenchMcpDependency, ApplicationWorkbenchServiceDependency,
    ApplicationWorkbenchSkillBundleDependency, ApplicationWorkbenchUiSurfaceDeclaration,
    ExecutionControlCheckpointMode, ExecutionControlMode, ExecutionControlPolicy,
    ExecutionControlResumeSource, ExecutionControlTrigger,
};

#[test]
fn application_manifest_v1_roundtrips() {
    let manifest = ApplicationManifestV1::new(
        PackageId::new("application.fixture"),
        DeveloperId::new("developer.fixture"),
        "Fixture Application",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .ability(ApplicationAbilityDescriptor::new(
        "ability.fixture.agent",
        ApplicationAbilityKind::Agent,
        AbilityImplementationKind::Declarative,
    ))
    .permission(ApplicationPermissionDeclaration::required(
        "trace.emit",
        "Fixture emits trace events",
    ));

    let encoded = serde_json::to_string(&manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, manifest);
}

#[test]
fn application_manifest_v1_roundtrips_execution_control_policy() {
    let policy = ExecutionControlPolicy::enabled(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    )
    .allow_command_overrides(true);

    let manifest = ApplicationManifestV1::new(
        PackageId::new("application.execution-control"),
        DeveloperId::new("developer.fixture"),
        "Execution Control Application",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .execution_control(policy);

    let encoded = serde_json::to_string(&manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();
    let decoded_policy = decoded.execution_control.unwrap();

    assert_eq!(decoded_policy.mode, ExecutionControlMode::Enabled);
    assert!(decoded_policy.allow_command_overrides);
    assert_eq!(decoded_policy.triggers.len(), 1);
    assert_eq!(decoded_policy.resume_sources.len(), 1);
}

#[test]
fn application_manifest_v1_roundtrips_tool_planning_policy() {
    let manifest = ApplicationManifestV1::new(
        PackageId::new("application.tool.policy.fixture"),
        DeveloperId::new("developer.fixture"),
        "Tool Policy Fixture",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .tool_family("web")
    .tool_family("memory")
    .toolset("research")
    .allowed_tool("read_file");

    let encoded = serde_json::to_string(&manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.tool_families, vec!["web", "memory"]);
    assert_eq!(decoded.toolsets, vec!["research"]);
    assert_eq!(decoded.allowed_tools, vec!["read_file"]);
}

#[test]
fn application_manifest_v1_declares_code_git_and_review_tool_families() {
    let manifest = ApplicationManifestV1::new(
        PackageId::new("application.workbench.policy.fixture"),
        DeveloperId::new("developer.fixture"),
        "Workbench Policy Fixture",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .tool_family("code_intelligence")
    .tool_family("git")
    .tool_family("review")
    .toolset("industrial.proof");

    let encoded = serde_json::to_string(&manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(
        decoded.tool_families,
        vec!["code_intelligence", "git", "review"]
    );
    assert_eq!(decoded.toolsets, vec!["industrial.proof"]);
}

#[test]
fn application_manifest_v1_roundtrips_sandbox_policy_declarations() {
    let manifest = ApplicationManifestV1::new(
        PackageId::new("application.sandbox.policy.fixture"),
        DeveloperId::new("developer.fixture"),
        "Sandbox Policy Fixture",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .permission_profile(WORKSPACE_WRITE_PROFILE)
    .sandbox_runtime_kind(SandboxRuntimeKind::LocalSandbox);

    let encoded = serde_json::to_string(&manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.permission_profiles, vec![WORKSPACE_WRITE_PROFILE]);
    assert_eq!(
        decoded.sandbox_runtime_kinds,
        vec![SandboxRuntimeKind::LocalSandbox]
    );
}

#[test]
fn application_manifest_v1_roundtrips_workbench_declaration() {
    let workbench = ApplicationWorkbenchManifestDeclaration {
        capabilities: vec![ApplicationWorkbenchCapabilityDeclaration::required(
            "file",
            "Application inspects declared workspace inputs",
        )],
        permission_profiles: vec![WORKSPACE_WRITE_PROFILE.into()],
        tool_families: vec!["review".into()],
        service_dependencies: vec![ApplicationWorkbenchServiceDependency::required(
            crate::KernelServiceId::new("service.review"),
            "Application requests structured review through service boundary",
        )],
        mcp_dependencies: vec![ApplicationWorkbenchMcpDependency {
            server_id: "mcp.generic".into(),
            lifecycle_scope: "session".into(),
            optional: true,
            reason: "Optional contextual tools".into(),
        }],
        skill_bundles: vec![ApplicationWorkbenchSkillBundleDependency {
            bundle_id: "skill.bundle.generic".into(),
            version_req: None,
            optional: true,
            reason: "Optional reusable procedures".into(),
        }],
        event_subscriptions: vec![ApplicationWorkbenchEventSubscription {
            topic: "thread.item".into(),
            optional: false,
            reason: "Application protocol streams item updates".into(),
        }],
        ui_surfaces: vec![ApplicationWorkbenchUiSurfaceDeclaration {
            surface_id: "main".into(),
            schema: "genui.v1".into(),
            mode: "workspace".into(),
            required_bridge_capabilities: vec!["service.call".into()],
            event_subscriptions: vec!["thread.item".into()],
            metadata: Default::default(),
        }],
        ..Default::default()
    };
    let manifest = ApplicationManifestV1::new(
        PackageId::new("application.workbench.manifest.fixture"),
        DeveloperId::new("developer.fixture"),
        "Workbench Manifest Fixture",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .ability(ApplicationAbilityDescriptor::new(
        "ability.fixture.agent",
        ApplicationAbilityKind::Agent,
        AbilityImplementationKind::Declarative,
    ))
    .workbench(workbench);

    let encoded = serde_json::to_string(&manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();
    let decoded_workbench = decoded.workbench.unwrap();

    assert_eq!(decoded_workbench.capabilities[0].family, "file");
    assert_eq!(
        decoded_workbench.permission_profiles,
        vec![WORKSPACE_WRITE_PROFILE]
    );
    assert_eq!(
        decoded_workbench.event_subscriptions[0].topic,
        "thread.item"
    );
}
