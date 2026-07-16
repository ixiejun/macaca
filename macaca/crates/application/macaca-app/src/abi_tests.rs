use std::sync::Arc;

use macaca_proto::{
    knowledge_retrieval_pack_definition, knowledge_summarization_pack_definition,
    ApplicationImport, DeveloperId, DomainPackAvailability, InMemoryDomainPackCatalog,
    PackageDescriptor, PackageId, PackageManifest, PackageRuntime, PackageRuntimeKind, PackageType,
};

use super::*;
use crate::loader::AppLoader;

fn first_example_app() -> std::path::PathBuf {
    let examples_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/apps");
    let mut paths = std::fs::read_dir(examples_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("app.yaml"))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter().next().unwrap()
}

#[test]
fn application_abi_yaml_adapter_preserves_manifest_metadata() {
    let manifest = AppLoader::load_manifest(first_example_app()).unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(manifest.clone())
        .load()
        .unwrap()
        .descriptor;

    assert_eq!(descriptor.runtime_kind, Some(PackageRuntimeKind::Yaml));
    assert_eq!(
        descriptor
            .declaration
            .metadata
            .get("application.name")
            .unwrap(),
        &manifest.name
    );
    descriptor.declaration.validate_required_exports().unwrap();
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::TaskCreateGoal));
}

#[test]
fn application_abi_yaml_projection_preserves_key_fields() {
    let manifest = AppLoader::load_manifest(first_example_app()).unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(manifest.clone())
        .load()
        .unwrap()
        .descriptor;

    assert_eq!(
        descriptor.declaration.application_id,
        manifest.id.to_string()
    );
    assert_eq!(descriptor.runtime_kind, Some(PackageRuntimeKind::Yaml));
    assert!(descriptor.declaration.package_id.is_some());
    assert_eq!(
        descriptor
            .declaration
            .metadata
            .get("manifest.version")
            .map(String::as_str),
        Some("1")
    );
    let expected_ability_count = manifest.agents.len().to_string();
    assert_eq!(
        descriptor.metadata.get("ability.count").map(String::as_str),
        Some(expected_ability_count.as_str())
    );
}

#[test]
fn application_abi_wasm_adapter_loads_metadata_but_not_execution() {
    let package = PackageDescriptor::new(PackageManifest::new(
        PackageId::new("pkg.wasm"),
        PackageType::Application,
        "1.0.0",
        DeveloperId::new("dev.wasm"),
        PackageRuntime::new(PackageRuntimeKind::WasmComponent, "0"),
    ));
    let adapter = WasmApplicationAbiAdapter::new(package);
    let load = adapter.load().unwrap();
    assert_eq!(
        load.descriptor.runtime_kind,
        Some(PackageRuntimeKind::WasmComponent)
    );
    assert!(is_runtime_unavailable(&adapter.execute_unavailable()));
}

#[test]
fn application_abi_projects_declared_summary_pack_as_generic_service_capability() {
    let manifest = summary_manifest("summary-abi-fixture", true);
    let mut installed_catalog = InMemoryDomainPackCatalog::new();
    let mut summarization = knowledge_summarization_pack_definition();
    // A composition root marks the descriptor callable only after a provider is installed.
    summarization.metadata.availability = DomainPackAvailability::Available;
    installed_catalog.register(summarization);
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(installed_catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = summary_projection(&descriptor);

    assert!(projection
        .callable_commands
        .contains("summarization.summarize"));
    assert!(descriptor
        .declaration
        .permissions
        .contains(&"summarization.run".into()));
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
    assert!(projection.unavailable_commands.is_empty());
}

#[test]
fn application_abi_surfaces_unavailable_summary_commands_from_injected_catalog() {
    let mut summarization = knowledge_summarization_pack_definition();
    summarization.metadata.availability = DomainPackAvailability::PreviewUnavailable;
    summarization.metadata.diagnostics.unavailable_reason =
        "summarization_runtime_not_installed".into();
    let mut unavailable_catalog = InMemoryDomainPackCatalog::new();
    unavailable_catalog.register(summarization);
    let descriptor =
        YamlApplicationAbiAdapter::new(summary_manifest("summary-unavailable-abi-fixture", false))
            .with_catalog(Arc::new(unavailable_catalog))
            .load()
            .unwrap()
            .descriptor;
    let projection = summary_projection(&descriptor);

    assert!(projection.callable_commands.is_empty());
    assert_eq!(
        projection
            .unavailable_commands
            .get("summarization.summarize")
            .map(String::as_str),
        Some("summarization_runtime_not_installed")
    );
}

#[test]
fn application_abi_projects_declared_retrieval_scopes_and_command_schemas() {
    let manifest = AppLoader::parse_manifest_yaml(
        r#"
name: retrieval-abi-fixture
layer: L2Wasm
service_contract:
  optional_packs:
    - pack.knowledge.retrieval.v1
  pack_permission_scopes:
    pack.knowledge.retrieval.v1:
      - retrieval.collection.manage
      - retrieval.record.write
      - retrieval.query
      - retrieval.rerank
      - retrieval.read
      - retrieval.evidence
"#,
    )
    .unwrap();
    let mut retrieval = knowledge_retrieval_pack_definition();
    retrieval.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(retrieval);

    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.knowledge.retrieval.v1")
        .expect("declared retrieval pack must produce an ABI projection");

    for command in [
        "retrieval.register_collection",
        "retrieval.upsert_records",
        "retrieval.retrieve",
        "retrieval.rerank_context",
        "retrieval.expand_context",
        "retrieval.package_evidence",
    ] {
        assert!(projection.callable_commands.contains(command));
    }
    assert!(descriptor
        .declaration
        .permissions
        .contains(&"retrieval.record.write".into()));
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
}

fn summary_manifest(name: &str, includes_scopes: bool) -> crate::model::AppManifest {
    let scopes = if includes_scopes {
        "\n  pack_permission_scopes:\n    pack.knowledge.summarization.v1:\n      - summarization.plan\n      - summarization.run"
    } else {
        ""
    };
    AppLoader::parse_manifest_yaml(&format!(
        "name: {name}\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.knowledge.summarization.v1{scopes}\n"
    )).unwrap()
}

fn summary_projection(
    descriptor: &ApplicationAbiDescriptor,
) -> &macaca_proto::DomainPackEffectiveCapabilityProjection {
    descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.knowledge.summarization.v1")
        .expect("declared summary pack must produce an ABI discovery projection")
}
