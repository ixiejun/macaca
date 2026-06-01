use macaca_app::{
    app_manifest_to_metadata_view, ApplicationAdmissionReport, ApplicationManifestV1Spec,
    YamlApplicationManifestAdapter,
};
use macaca_proto::{
    AbilityImplementationKind, ApplicationAbilityDescriptor, ApplicationAbilityKind,
    ApplicationCompatibilityDeclaration, ApplicationExecutionControlKind,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionProfileDeclaration,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderPreference,
    ApplicationManifestV1, ApplicationRuntimeProfile, ApplicationWorkbenchCapabilityDeclaration,
    ApplicationWorkbenchEventSubscription, ApplicationWorkbenchManifestDeclaration,
    ApplicationWorkbenchMcpDependency, ApplicationWorkbenchOptionalProviderRequirement,
    ApplicationWorkbenchPluginDependency, ApplicationWorkbenchServiceDependency,
    ApplicationWorkbenchSkillBundleDependency, ApplicationWorkbenchUiSurfaceDeclaration,
    CapabilityId, DeveloperId, ExternalApplicationBackendExecutionProfile, KernelServiceId,
    PackageId, PackageRuntimeKind,
};

fn workbench_declaration() -> ApplicationWorkbenchManifestDeclaration {
    ApplicationWorkbenchManifestDeclaration {
        capabilities: vec![
            ApplicationWorkbenchCapabilityDeclaration::required(
                "file",
                "Application reads declared workspace state through service.file",
            ),
            ApplicationWorkbenchCapabilityDeclaration::required(
                "process",
                "Application runs declared commands through service.process",
            ),
            ApplicationWorkbenchCapabilityDeclaration::required(
                "review",
                "Application receives structured review findings",
            ),
        ],
        permission_profiles: vec!["workspace_write".into()],
        tool_families: vec!["code_intelligence".into(), "git".into(), "review".into()],
        service_dependencies: vec![
            ApplicationWorkbenchServiceDependency::required(
                KernelServiceId::new("service.file"),
                "Workspace file inspection is service-owned",
            )
            .capability(CapabilityId::new("capability.file.read")),
            ApplicationWorkbenchServiceDependency::required(
                KernelServiceId::new("service.app_protocol"),
                "Application subscribes through the app protocol gateway",
            ),
        ],
        optional_provider_requirements: vec![ApplicationWorkbenchOptionalProviderRequirement {
            provider_kind: "remote_environment".into(),
            capability: Some(CapabilityId::new("capability.remote_environment")),
            reason: "Remote workspaces are optional provider-backed environments".into(),
        }],
        plugin_dependencies: vec![ApplicationWorkbenchPluginDependency {
            plugin_id: "plugin.generic.example".into(),
            version_req: Some("^1".into()),
            optional: true,
            reason: "Optional plugin extends declared tool catalog".into(),
        }],
        mcp_dependencies: vec![ApplicationWorkbenchMcpDependency {
            server_id: "mcp.generic.example".into(),
            lifecycle_scope: "session".into(),
            optional: true,
            reason: "Optional MCP server contributes declared tools".into(),
        }],
        skill_bundles: vec![ApplicationWorkbenchSkillBundleDependency {
            bundle_id: "skill.bundle.generic.example".into(),
            version_req: None,
            optional: true,
            reason: "Optional skill bundle contributes reusable procedures".into(),
        }],
        event_subscriptions: vec![
            ApplicationWorkbenchEventSubscription {
                topic: "thread.item".into(),
                optional: false,
                reason: "Thread items stream through app protocol".into(),
            },
            ApplicationWorkbenchEventSubscription {
                topic: "process.output".into(),
                optional: false,
                reason: "Process output streams through app protocol".into(),
            },
        ],
        ui_surfaces: vec![ApplicationWorkbenchUiSurfaceDeclaration {
            surface_id: "main".into(),
            schema: "genui.v1".into(),
            mode: "workspace".into(),
            required_bridge_capabilities: vec!["service.call".into()],
            event_subscriptions: vec!["thread.item".into()],
            metadata: Default::default(),
        }],
        metadata: Default::default(),
    }
}

fn manifest_for(
    kind: PackageRuntimeKind,
    ability_kind: ApplicationAbilityKind,
) -> ApplicationManifestV1 {
    ApplicationManifestV1::new(
        PackageId::new(format!("application.workbench.{kind}")),
        DeveloperId::new("developer.fixture"),
        format!("Workbench {kind}"),
        "1.0.0",
        ApplicationRuntimeProfile::new(kind, "1"),
        ApplicationCompatibilityDeclaration::new("0.1.0"),
    )
    .ability(ApplicationAbilityDescriptor::new(
        "ability.workbench.generic",
        ability_kind,
        AbilityImplementationKind::Declarative,
    ))
    .workbench(workbench_declaration())
}

fn assert_admitted(report: ApplicationAdmissionReport) {
    assert!(
        report.is_success(),
        "unexpected diagnostics: {:?}",
        report.diagnostics
    );
}

fn external_execution_profile() -> ApplicationExecutionProfileDeclaration {
    ApplicationExecutionProfileDeclaration {
        provider_preference: Some(ApplicationExecutionProviderPreference {
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            provider_id: Some("provider.external.fixture".into()),
        }),
        external_backend: Some(ExternalApplicationBackendExecutionProfile {
            provider_id: "provider.external.fixture".into(),
            start_endpoint: "https://backend.example.test/start".into(),
            control_endpoint: Some("https://backend.example.test/control".into()),
            protocol_version: "application-execution.v1".into(),
            callback_gateway_ref: Some("gateway/application-execution".into()),
            callback_identity_ref: "identity.external.fixture".into(),
            supported_controls: vec![ApplicationExecutionControlKind::Cancel],
            heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
                interval_ms: 1000,
                timeout_ms: 5000,
                required: true,
            },
            request_timeout_ms: 3000,
            event_schema_version: "application-execution.v1".into(),
            capability_declarations: vec![CapabilityId::new("capability.application_execution")],
            resource_profile: Default::default(),
        }),
        metadata: Default::default(),
    }
}

#[test]
fn yaml_manifest_projects_workbench_declarations_to_manifest_v1_and_metadata() {
    let yaml = r#"
name: generic-workbench
version: "1.0.0"
layer: L3Declarative
agents:
  - name: coordinator
    prompt_template: "raw prompt must not leak"
workbench:
  capabilities:
    - family: file
      optional: false
      reason: Reads declared workspace state through service.file
  permission_profiles: [workspace_write]
  tool_families: [code_intelligence, git, review]
  service_dependencies:
    - service: service.file
      optional: false
      reason: Workspace file inspection is service-owned
  plugin_dependencies:
    - plugin_id: plugin.generic.example
      optional: true
      reason: Optional plugin extends declared tool catalog
  mcp_dependencies:
    - server_id: mcp.generic.example
      lifecycle_scope: session
      optional: true
      reason: Optional MCP server contributes declared tools
  skill_bundles:
    - bundle_id: skill.bundle.generic.example
      optional: true
      reason: Optional skill bundle contributes reusable procedures
  event_subscriptions:
    - topic: thread.item
      optional: false
      reason: Thread items stream through app protocol
  ui_surfaces:
    - surface_id: main
      schema: genui.v1
      mode: workspace
      required_bridge_capabilities: [service.call]
      event_subscriptions: [thread.item]
"#;
    let legacy: macaca_app::AppManifest = serde_yaml::from_str(yaml).unwrap();
    let projection = YamlApplicationManifestAdapter::new(legacy.clone()).project();

    assert_admitted(ApplicationManifestV1Spec.validate(&projection.manifest));
    assert_eq!(
        projection
            .manifest
            .workbench
            .as_ref()
            .unwrap()
            .tool_families,
        vec!["code_intelligence", "git", "review"]
    );

    let metadata = app_manifest_to_metadata_view(
        &legacy,
        None,
        macaca_app::AppStatus::Loaded,
        true,
        true,
        true,
        true,
    );
    assert_eq!(metadata.workbench.declared_capabilities, vec!["file"]);
    assert_eq!(metadata.workbench.event_subscriptions, vec!["thread.item"]);
    assert!(!serde_json::to_string(&metadata)
        .unwrap()
        .contains("raw prompt"));
}

#[test]
fn multiple_application_styles_admit_same_generic_workbench_capabilities() {
    let cases = vec![
        manifest_for(PackageRuntimeKind::Yaml, ApplicationAbilityKind::Agent),
        manifest_for(
            PackageRuntimeKind::WasmComponent,
            ApplicationAbilityKind::Headless,
        ),
        manifest_for(
            PackageRuntimeKind::NativeAdapter,
            ApplicationAbilityKind::Ui,
        ),
        manifest_for(
            PackageRuntimeKind::RemoteService,
            ApplicationAbilityKind::Headless,
        ),
        manifest_for(
            PackageRuntimeKind::Custom("workbench_style".into()),
            ApplicationAbilityKind::Extension,
        ),
    ];

    for manifest in cases {
        assert_admitted(ApplicationManifestV1Spec.validate(&manifest));
        let workbench = manifest.workbench.as_ref().unwrap();
        assert!(workbench
            .service_dependencies
            .iter()
            .any(|dependency| dependency.service.as_str() == "service.file"));
        assert!(workbench
            .event_subscriptions
            .iter()
            .any(|subscription| subscription.topic == "thread.item"));
    }
}

#[test]
fn manifest_v1_admits_external_backend_execution_profile() {
    let manifest = manifest_for(
        PackageRuntimeKind::RemoteService,
        ApplicationAbilityKind::Headless,
    )
    .execution_profile(external_execution_profile());

    assert_admitted(ApplicationManifestV1Spec.validate(&manifest));
    let execution_profile = manifest.execution_profile.as_ref().unwrap();
    assert_eq!(
        execution_profile
            .external_backend
            .as_ref()
            .unwrap()
            .callback_identity_ref,
        "identity.external.fixture"
    );
}

#[test]
fn yaml_projection_preserves_external_backend_execution_profile() {
    let yaml = r#"
name: generic-external-backend-app
version: "1.0.0"
layer: L3Declarative
agents:
  - name: coordinator
    prompt_template: "raw prompt must not leak"
execution_profile:
  provider_preference:
    provider_kind: ExternalAppBackend
    provider_id: provider.external.fixture
  external_backend:
    provider_id: provider.external.fixture
    start_endpoint: https://backend.example.test/start
    control_endpoint: https://backend.example.test/control
    protocol_version: application-execution.v1
    callback_gateway_ref: gateway/application-execution
    callback_identity_ref: identity.external.fixture
    supported_controls: [Cancel]
    heartbeat_policy:
      interval_ms: 1000
      timeout_ms: 5000
      required: true
    request_timeout_ms: 3000
    event_schema_version: application-execution.v1
    capability_declarations:
      - capability.application_execution
"#;
    let legacy: macaca_app::AppManifest = serde_yaml::from_str(yaml).unwrap();
    let projection = YamlApplicationManifestAdapter::new(legacy).project();

    assert_admitted(ApplicationManifestV1Spec.validate(&projection.manifest));
    assert_eq!(
        projection
            .manifest
            .execution_profile
            .as_ref()
            .unwrap()
            .external_backend
            .as_ref()
            .unwrap()
            .provider_id,
        "provider.external.fixture"
    );
}
