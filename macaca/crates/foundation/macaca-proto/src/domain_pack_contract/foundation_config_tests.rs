use std::collections::{BTreeMap, BTreeSet};

use super::*;

// The config pack tests validate provider-neutral metadata only. They do not
// read environment variables, package descriptors, remote config, tenant config,
// or raw values; provider interaction belongs behind the service runtime.

#[test]
fn foundation_config_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_config_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_CONFIG_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "config_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/config"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_CONFIG_SERVICE_ID)
        .expect("config descriptor exposes command schemas");
    for command in FOUNDATION_CONFIG_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "config.read",
        "config.list",
        "config.validate",
        "config.watch",
        "config.reload",
        "config.snapshot",
        "config.export",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in [
        "package-descriptor",
        "workspace",
        "environment",
        "remote",
        "mock",
        "unavailable",
    ] {
        assert!(
            definition
                .metadata
                .provider_descriptors
                .contains_key(provider_class),
            "missing provider descriptor {provider_class}"
        );
    }
}

#[test]
fn industrial_catalog_uses_foundation_config_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_CONFIG_PACK_ID)
        .expect("industrial catalog includes foundation config");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_CONFIG_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("environment")
            .and_then(|descriptor| descriptor.metadata.get("reload"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_config_command_dtos_are_serde_compatible() {
    let schema = ConfigSchemaReference {
        schema_id: "app.settings".into(),
        version: "v1".into(),
    };
    let key = ConfigKeyReference {
        key: "ui.theme".into(),
        namespace: "app".into(),
    };
    let selector = ConfigSelector {
        profile: "default".into(),
        tenant_ref: Some("tenant-ref".into()),
        environment_ref: Some("env-ref".into()),
    };
    let source = ConfigSourceReference {
        source_id: "workspace".into(),
        provider_class: "workspace".into(),
        redacted_location_hash: "location-hash".into(),
    };

    let commands = vec![
        serde_json::to_value(ConfigDescribeSchemaCommand {
            schema: schema.clone(),
        })
        .unwrap(),
        serde_json::to_value(ConfigGetCommand {
            key: key.clone(),
            selector: selector.clone(),
        })
        .unwrap(),
        serde_json::to_value(ConfigGetManyCommand {
            keys: vec![key.clone()],
            selector: selector.clone(),
        })
        .unwrap(),
        serde_json::to_value(ConfigListKeysCommand {
            namespace: "app".into(),
            prefix: Some("ui.".into()),
            page_size: 50,
            cursor: None,
        })
        .unwrap(),
        serde_json::to_value(ConfigResolveEffectiveCommand {
            key: key.clone(),
            selector: selector.clone(),
            include_provenance: true,
        })
        .unwrap(),
        serde_json::to_value(ConfigValidateCommand {
            candidate_ref: "artifact:candidate-config".into(),
            schema: schema.clone(),
            selector: selector.clone(),
        })
        .unwrap(),
        serde_json::to_value(ConfigExplainProvenanceCommand {
            key: key.clone(),
            selector: selector.clone(),
        })
        .unwrap(),
        serde_json::to_value(ConfigWatchCommand {
            namespace: "app".into(),
            selector: selector.clone(),
            start_cursor: Some("cursor".into()),
        })
        .unwrap(),
        serde_json::to_value(ConfigReloadCommand {
            source,
            dry_run: true,
        })
        .unwrap(),
        serde_json::to_value(ConfigSnapshotCommand {
            selector: selector.clone(),
            include_values: false,
        })
        .unwrap(),
        serde_json::to_value(ConfigExportRedactedCommand {
            selector,
            redaction_level: "metadata_only".into(),
        })
        .unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_CONFIG_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_config_hashes_change_with_contract_content() {
    let request = ConfigListKeysCommand {
        namespace: "app".into(),
        prefix: Some("ui.".into()),
        page_size: 50,
        cursor: None,
    };
    let mut changed = request.clone();
    changed.page_size = 100;

    assert_eq!(config_stable_hash(&request), config_stable_hash(&request));
    assert_ne!(config_stable_hash(&request), config_stable_hash(&changed));

    let hashes = foundation_config_descriptor_hashes();
    let unique = BTreeSet::from([
        hashes.command_schema_hash,
        hashes.result_schema_hash,
        hashes.snapshot_schema_hash,
        hashes.provider_capability_schema_hash,
        hashes.unavailable_schema_hash,
    ]);
    assert_eq!(unique.len(), 5);
    assert!(unique.iter().all(|hash| !hash.is_empty()));
}

#[test]
fn foundation_config_result_and_snapshot_dtos_are_bounded() {
    let redaction = ConfigRedactionSummary {
        redacted_value_count: 3,
        redacted_source_count: 1,
        contains_secret_references: true,
    };
    let snapshot = ConfigProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        source_hashes: BTreeMap::from([("workspace".into(), "source-hash".into())]),
        schema_hashes: BTreeMap::from([("app.settings".into(), "schema-hash".into())]),
        redaction_summary: redaction.clone(),
    };
    let unavailable: ConfigResultEnvelope<String> = ConfigResultEnvelope {
        status: ConfigResultStatus::Unavailable,
        data: None,
        error: Some(ConfigError {
            code: ConfigResultStatus::Unavailable,
            message: "config provider is not installed".into(),
            retryable: false,
        }),
        trace_id: "trace-config-unavailable".into(),
        descriptor_hash: config_stable_hash(&snapshot),
    };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-config-unavailable"));
    assert!(!serialized.contains("DATABASE_PASSWORD"));
    assert!(!serialized.contains("raw-secret-value"));
    assert_eq!(snapshot.redaction_summary, redaction);
}
