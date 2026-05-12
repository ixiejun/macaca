use macaca_app::{
    application_manifest_v1_to_package_descriptor, load_yaml_app_package_descriptor, AppLoader,
    PackageGuardContext, PackageLoaderFactory, PackageRuntimeGuard, YamlApplicationManifestAdapter,
};
use macaca_proto::{KernelServiceId, PackageGuardDecision, PackageRuntimeKind};

fn example_apps() -> Vec<std::path::PathBuf> {
    let examples_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/apps");
    let mut paths = std::fs::read_dir(examples_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("app.yaml"))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[test]
fn package_manifest_v1_projection_preserves_legacy_key_fields() {
    let app_path = example_apps().into_iter().next().unwrap();
    let legacy_descriptor = load_yaml_app_package_descriptor(&app_path).unwrap();
    let manifest = AppLoader::load_manifest(&app_path).unwrap();
    let projection = YamlApplicationManifestAdapter::new(manifest).project();
    let projected_descriptor = application_manifest_v1_to_package_descriptor(&projection.manifest);

    assert_eq!(
        legacy_descriptor.manifest.id,
        projected_descriptor.manifest.id
    );
    assert_eq!(
        legacy_descriptor.manifest.runtime.kind,
        Some(PackageRuntimeKind::Yaml)
    );
    assert_eq!(
        legacy_descriptor.manifest.runtime.kind,
        projected_descriptor.manifest.runtime.kind
    );
    assert_eq!(
        legacy_descriptor.manifest.entry,
        projected_descriptor.manifest.entry
    );
    assert_eq!(
        legacy_descriptor.manifest.provides.len(),
        projected_descriptor.manifest.provides.len()
    );
}

#[test]
fn package_manifest_yaml_apps_load_as_packages() {
    let descriptors = example_apps()
        .into_iter()
        .map(load_yaml_app_package_descriptor)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(!descriptors.is_empty());
    assert!(descriptors.iter().all(|descriptor| descriptor
        .manifest
        .metadata
        .contains_key("application.name")));
    assert!(descriptors.iter().any(|descriptor| descriptor
        .manifest
        .provides
        .iter()
        .any(|capability| capability.id.as_str().starts_with("tool."))));
}

#[test]
fn package_manifest_yaml_apps_preserve_agent_capabilities() {
    let descriptors = example_apps()
        .into_iter()
        .map(load_yaml_app_package_descriptor)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(descriptors.iter().any(|descriptor| descriptor
        .manifest
        .provides
        .iter()
        .any(|capability| capability.id.as_str().contains("agent."))));
}

#[test]
fn package_manifest_runtime_guard_accepts_yaml_app_when_required_service_exists() {
    let descriptor =
        PackageLoaderFactory::load_yaml_application(example_apps().into_iter().next().unwrap())
            .unwrap();
    let context =
        PackageGuardContext::new("1").with_service(KernelServiceId::new("service.agent.runtime"));

    let guarded = PackageRuntimeGuard::new().evaluate(descriptor, &context);

    assert!(matches!(
        guarded.decision,
        PackageGuardDecision::Accepted
            | PackageGuardDecision::AcceptedWithUnavailableOptionalServices
    ));
}
