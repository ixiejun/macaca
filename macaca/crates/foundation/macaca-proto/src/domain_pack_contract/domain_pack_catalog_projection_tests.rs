use std::collections::BTreeSet;

use super::*;
#[test]
fn every_industrial_preview_pack_projects_unavailable_commands_and_replay_refs() {
    let definitions = industrial_reference_domain_pack_definitions();
    let catalog = compose_installed_domain_pack_catalog(definitions.clone());

    for definition in definitions {
        let expanded = expand_service_capabilities(
            Some(&AppServiceContractConfig {
                required_packs: vec![definition.pack_id.clone()],
                ..Default::default()
            }),
            catalog.as_ref(),
        );
        let projections = expanded
            .capability_projections
            .iter()
            .filter(|projection| projection.pack_id == definition.pack_id)
            .collect::<Vec<_>>();

        assert!(
            !projections.is_empty(),
            "missing effective capability projection for {}",
            definition.pack_id
        );
        for projection in projections {
            assert!(projection.callable_commands.is_empty());
            assert!(projection.denied_commands.is_empty());
            assert!(!projection.replay_refs.is_empty());
            assert!(projection
                .replay_refs
                .contains(&definition.stable_descriptor_hash()));
            assert!(projection.unavailable_features.contains_key("pack"));

            let descriptor_commands = definition
                .metadata
                .service_command_schemas
                .get(&projection.service_id)
                .cloned()
                .unwrap_or_default();
            let unavailable_commands = projection
                .unavailable_commands
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            assert_eq!(unavailable_commands, descriptor_commands);
            assert!(
                !projection.provider_capability_flags.is_empty(),
                "{} should expose provider flags for diagnostics",
                definition.pack_id
            );
        }
    }
}
