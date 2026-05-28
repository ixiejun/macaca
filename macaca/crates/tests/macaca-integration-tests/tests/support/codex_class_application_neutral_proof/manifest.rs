//! Manifest fixture for the application-neutral Codex-class proof.
//!
//! The manifest is data-only. It declares generic service dependencies,
//! capabilities, tool families, and event subscriptions so the proof never
//! relies on package-name inference or OS-layer product branches.

use std::collections::BTreeMap;

use macaca_proto::workbench::{
    app_protocol as app, approval, diagnostics as diag, file, git, hook, interaction, process,
    review, sandbox,
};
use macaca_proto::{
    ApplicationCompatibilityDeclaration, ApplicationManifestV1, ApplicationRuntimeProfile,
    ApplicationWorkbenchCapabilityDeclaration, ApplicationWorkbenchEventSubscription,
    ApplicationWorkbenchManifestDeclaration, ApplicationWorkbenchMcpDependency,
    ApplicationWorkbenchServiceDependency, ApplicationWorkbenchSkillBundleDependency,
    ApplicationWorkbenchUiSurfaceDeclaration, CapabilityId, DeveloperId, KernelServiceId,
    PackageId, PackageRuntimeKind, APPLICATION_SERVICE_ID, MCP_SERVICE_ID, TOOL_SERVICE_ID,
};

pub fn proof_manifest() -> ApplicationManifestV1 {
    let services = [
        APPLICATION_SERVICE_ID,
        interaction::SERVICE_ID,
        app::SERVICE_ID,
        file::SERVICE_ID,
        macaca_proto::workbench::code_intelligence::SERVICE_ID,
        git::SERVICE_ID,
        sandbox::SERVICE_ID,
        process::SERVICE_ID,
        approval::SERVICE_ID,
        hook::SERVICE_ID,
        TOOL_SERVICE_ID,
        review::SERVICE_ID,
        diag::SERVICE_ID,
        MCP_SERVICE_ID,
    ];
    let families = [
        "file",
        "sandbox",
        "shell",
        "approval",
        "hook",
        "mcp",
        "skill",
        "code_intelligence",
        "git",
        "review",
        "diagnostics",
    ];
    let mut declaration = ApplicationWorkbenchManifestDeclaration::default();
    declaration.capabilities = families
        .iter()
        .map(|family| {
            ApplicationWorkbenchCapabilityDeclaration::required(
                *family,
                "generic proof capability declared by manifest data",
            )
        })
        .collect();
    declaration.permission_profiles = vec![sandbox::WORKSPACE_WRITE_PROFILE.into()];
    declaration.tool_families = families.iter().map(|family| family.to_string()).collect();
    declaration.service_dependencies = services
        .iter()
        .map(|service| {
            ApplicationWorkbenchServiceDependency::required(
                KernelServiceId::new(*service),
                "proof fixture uses this generic service boundary",
            )
            .capability(CapabilityId::new(format!("capability.proof.{service}")))
        })
        .collect();
    declaration.mcp_dependencies = vec![ApplicationWorkbenchMcpDependency {
        server_id: "mcp.generic-proof".into(),
        lifecycle_scope: "session".into(),
        optional: true,
        reason: "optional MCP dependency proves declaration shape".into(),
    }];
    declaration.skill_bundles = vec![ApplicationWorkbenchSkillBundleDependency {
        bundle_id: "skill.generic-proof".into(),
        version_req: None,
        optional: true,
        reason: "optional skill bundle proves tool-family dispatch".into(),
    }];
    declaration.event_subscriptions = services
        .iter()
        .map(|service| ApplicationWorkbenchEventSubscription {
            topic: format!("{service}.events.v1"),
            optional: false,
            reason: "proof streams bounded service events through app protocol".into(),
        })
        .collect();
    declaration.ui_surfaces = vec![ApplicationWorkbenchUiSurfaceDeclaration {
        surface_id: "surface.generic-workbench-proof".into(),
        schema: "genui.workbench.proof.v1".into(),
        mode: "workspace".into(),
        required_bridge_capabilities: vec!["app_protocol".into()],
        event_subscriptions: declaration
            .event_subscriptions
            .iter()
            .map(|subscription| subscription.topic.clone())
            .collect(),
        metadata: BTreeMap::new(),
    }];

    ApplicationManifestV1::new(
        PackageId::new("application.generic-workbench-proof"),
        DeveloperId::new("developer.macaca.fixture"),
        "Generic Workbench Proof",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
        ApplicationCompatibilityDeclaration::new("0.1.0"),
    )
    .workbench(declaration)
    .toolset("industrial.workbench")
    .permission_profile(sandbox::WORKSPACE_WRITE_PROFILE)
    .sandbox_runtime_kind(sandbox::SandboxRuntimeKind::LocalSandbox)
}

pub fn assert_manifest_declares_full_workbench_surface(manifest: &ApplicationManifestV1) {
    let encoded = serde_json::to_string(manifest).unwrap();
    let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();
    let workbench = decoded.workbench.as_ref().unwrap();
    assert!(workbench
        .service_dependencies
        .iter()
        .any(|dependency| dependency.service.as_str() == APPLICATION_SERVICE_ID));
    assert!(workbench
        .tool_families
        .iter()
        .any(|family| family == "skill"));
    assert!(workbench.tool_families.iter().any(|family| family == "mcp"));
    assert!(workbench.event_subscriptions.len() >= 10);
}
