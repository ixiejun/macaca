use std::collections::BTreeMap;

use super::*;

// These tests live outside tests.rs to keep each Rust source file below the
// repository constitution's 500-line ceiling. The module still executes through
// domain_pack_contract::tests so provider descriptor behavior is verified with
// the rest of the domain-pack contract suite.

#[test]
fn provider_descriptors_are_descriptor_owned_and_hash_visible() {
    let definition = DomainPackDefinition::with_metadata(
        "pack.device.camera.v1",
        DomainPackMetadata {
            family_id: "device".into(),
            parent_pack_id: Some("pack.device.v1".into()),
            version: "v1".into(),
            provider_descriptors: BTreeMap::from([
                (
                    "host-native".into(),
                    DomainPackProviderDescriptor::new(
                        "host-native",
                        "service.device.camera.host-native",
                    )
                    .with_availability(DomainPackProviderCapabilityState::Preview),
                ),
                (
                    "plugin".into(),
                    DomainPackProviderDescriptor::new("plugin", "service.device.camera.plugin")
                        .with_availability(DomainPackProviderCapabilityState::Available),
                ),
                (
                    "mock".into(),
                    DomainPackProviderDescriptor::new("mock", "service.device.camera.mock")
                        .with_availability(DomainPackProviderCapabilityState::Available),
                ),
                (
                    "unavailable".into(),
                    DomainPackProviderDescriptor::new(
                        "unavailable",
                        "service.device.camera.unavailable",
                    )
                    .with_availability(DomainPackProviderCapabilityState::Unavailable),
                ),
            ]),
            ..Default::default()
        },
        Vec::<String>::new(),
    );
    let same_definition = definition.clone();
    let mut changed_definition = definition.clone();
    changed_definition
        .metadata
        .provider_descriptors
        .get_mut("plugin")
        .unwrap()
        .capability_hash = "provider-capability-v2".into();

    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("mock")
            .unwrap()
            .provider_class,
        "mock"
    );
    assert_eq!(
        definition.stable_descriptor_hash(),
        same_definition.stable_descriptor_hash()
    );
    assert_ne!(
        definition.stable_descriptor_hash(),
        changed_definition.stable_descriptor_hash(),
        "provider descriptor changes must affect replayable descriptor evidence"
    );
}

#[test]
fn provider_descriptors_accept_common_replacement_class_labels() {
    let classes = [
        "built-in-durable",
        "remote-workflow-engine",
        "host-native",
        "browser",
        "remote-host",
        "embedded-tzdb",
        "boundary-lookup",
        "remote-api",
        "display-name",
        "plugin",
        "mock",
        "unavailable",
    ];
    let provider_descriptors = classes
        .iter()
        .map(|provider_class| {
            (
                (*provider_class).to_string(),
                DomainPackProviderDescriptor::new(
                    *provider_class,
                    format!("service.synthetic.{provider_class}"),
                )
                .with_availability(DomainPackProviderCapabilityState::Preview),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let definition = DomainPackDefinition::with_metadata(
        "pack.synthetic.provider-classes.v1",
        DomainPackMetadata {
            family_id: "synthetic".into(),
            version: "v1".into(),
            provider_descriptors,
            ..Default::default()
        },
        Vec::<String>::new(),
    );

    for provider_class in classes {
        assert!(
            definition
                .metadata
                .provider_descriptors
                .contains_key(provider_class),
            "missing provider descriptor class {provider_class}"
        );
    }
}
