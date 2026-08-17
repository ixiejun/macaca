use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[test]
fn filesystem_root_declarations_require_declared_pack_and_unique_safe_ids() {
    let root = FilesystemRootRef {
        root_id: "workspace".into(),
        root_kind: "app_workspace".into(),
    };
    assert!(
        validate_filesystem_root_declarations(&AppServiceContractConfig {
            optional_packs: vec![FOUNDATION_FILESYSTEM_PACK_ID.into()],
            filesystem_roots: vec![root.clone()],
            ..Default::default()
        })
        .is_ok()
    );
    assert!(
        validate_filesystem_root_declarations(&AppServiceContractConfig {
            filesystem_roots: vec![root.clone()],
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        validate_filesystem_root_declarations(&AppServiceContractConfig {
            optional_packs: vec![FOUNDATION_FILESYSTEM_PACK_ID.into()],
            filesystem_roots: vec![root.clone(), root],
            ..Default::default()
        })
        .is_err()
    );
}

// Filesystem tests validate provider-neutral metadata only. They do not touch
// host paths, open files, create watchers, or construct concrete providers.

#[test]
fn foundation_filesystem_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_filesystem_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_FILESYSTEM_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "filesystem_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/filesystem"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_FILESYSTEM_SERVICE_ID)
        .expect("filesystem descriptor exposes command schemas");
    for command in FOUNDATION_FILESYSTEM_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "filesystem.read",
        "filesystem.write",
        "filesystem.append",
        "filesystem.list",
        "filesystem.metadata",
        "filesystem.copy",
        "filesystem.move",
        "filesystem.delete",
        "filesystem.watch",
        "filesystem.temp",
        "filesystem.snapshot",
        "filesystem.restore",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in [
        "local-scoped-workspace",
        "wasi-preopen",
        "remote-artifact",
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
fn industrial_catalog_uses_foundation_filesystem_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_FILESYSTEM_PACK_ID)
        .expect("industrial catalog includes foundation filesystem");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_FILESYSTEM_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("local-scoped-workspace")
            .and_then(|descriptor| descriptor.metadata.get("atomic_write"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_filesystem_command_dtos_are_serde_compatible() {
    let root = FilesystemRootRef {
        root_id: "workspace".into(),
        root_kind: "app_workspace".into(),
    };
    let path = FilesystemPathRef {
        root: root.clone(),
        relative_path: "docs/readme.md".into(),
    };
    let handle = FilesystemHandleRef {
        handle_id: "handle-1".into(),
        root: root.clone(),
        access_mode: FilesystemAccessMode::Read,
        revision_id: Some("rev-1".into()),
    };
    let content = FilesystemContentRef {
        content_ref: "artifact:file-content".into(),
        encoding: Some("utf8".into()),
        expected_hash: Some("content-hash".into()),
    };
    let snapshot = FilesystemSnapshotRef {
        snapshot_id: "snapshot-1".into(),
        root: root.clone(),
        tree_hash: "tree-hash".into(),
    };

    let commands = vec![
        serde_json::to_value(FilesystemOpenHandleCommand {
            path: path.clone(),
            access_mode: FilesystemAccessMode::Read,
            conflict_mode: FilesystemConflictMode::Fail,
        })
        .unwrap(),
        serde_json::to_value(FilesystemCloseHandleCommand {
            handle: handle.clone(),
            reason: "complete".into(),
        })
        .unwrap(),
        serde_json::to_value(FilesystemReadFileCommand {
            path: Some(path.clone()),
            handle: None,
            range_start: 0,
            max_bytes: 4096,
        })
        .unwrap(),
        serde_json::to_value(FilesystemWriteFileCommand {
            path: path.clone(),
            content: content.clone(),
            conflict_mode: FilesystemConflictMode::Overwrite,
            atomic: true,
        })
        .unwrap(),
        serde_json::to_value(FilesystemAppendFileCommand {
            path: path.clone(),
            content,
        })
        .unwrap(),
        serde_json::to_value(FilesystemListDirectoryCommand {
            path: path.clone(),
            recursive: false,
            page_size: 100,
            cursor: None,
        })
        .unwrap(),
        serde_json::to_value(FilesystemStatPathCommand {
            path: path.clone(),
            follow_symlinks: false,
        })
        .unwrap(),
        serde_json::to_value(FilesystemCreateDirectoryCommand {
            path: path.clone(),
            recursive: true,
            conflict_mode: FilesystemConflictMode::CreateNew,
        })
        .unwrap(),
        serde_json::to_value(FilesystemCopyPathCommand {
            source: path.clone(),
            destination: path.clone(),
            recursive: true,
            conflict_mode: FilesystemConflictMode::Fail,
        })
        .unwrap(),
        serde_json::to_value(FilesystemMovePathCommand {
            source: path.clone(),
            destination: path.clone(),
            atomic_preferred: true,
            conflict_mode: FilesystemConflictMode::Overwrite,
        })
        .unwrap(),
        serde_json::to_value(FilesystemDeletePathCommand {
            path: path.clone(),
            recursive: true,
            tombstone: true,
        })
        .unwrap(),
        serde_json::to_value(FilesystemCreateTempCommand {
            root: root.clone(),
            namespace: "tmp".into(),
            ttl_seconds: Some(600),
        })
        .unwrap(),
        serde_json::to_value(FilesystemWatchPathCommand {
            path: path.clone(),
            recursive: true,
            event_filter: BTreeSet::from(["created".into(), "deleted".into()]),
        })
        .unwrap(),
        serde_json::to_value(FilesystemSnapshotTreeCommand {
            root: root.clone(),
            include_pattern: Some("**/*.md".into()),
            max_bytes: 1_048_576,
        })
        .unwrap(),
        serde_json::to_value(FilesystemRestoreSnapshotCommand {
            snapshot,
            target_root: root,
            conflict_mode: FilesystemConflictMode::Fail,
            dry_run: true,
        })
        .unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_FILESYSTEM_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_filesystem_hashes_change_with_contract_content() {
    let request = FilesystemListDirectoryCommand {
        path: FilesystemPathRef {
            root: FilesystemRootRef {
                root_id: "workspace".into(),
                root_kind: "app_workspace".into(),
            },
            relative_path: "docs".into(),
        },
        recursive: false,
        page_size: 100,
        cursor: None,
    };
    let mut changed = request.clone();
    changed.page_size = 200;

    assert_eq!(
        filesystem_stable_hash(&request),
        filesystem_stable_hash(&request)
    );
    assert_ne!(
        filesystem_stable_hash(&request),
        filesystem_stable_hash(&changed)
    );

    let hashes = foundation_filesystem_descriptor_hashes();
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
fn foundation_filesystem_result_and_snapshot_dtos_are_bounded() {
    let snapshot = FilesystemProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        open_handle_count: 0,
        active_watch_count: 0,
        root_hashes: BTreeMap::from([("workspace".into(), "root-hash".into())]),
    };
    let unavailable: FilesystemResultEnvelope<FilesystemMetadata> = FilesystemResultEnvelope {
        status: FilesystemResultStatus::Unavailable,
        data: None,
        error: Some(FilesystemError {
            code: FilesystemResultStatus::Unavailable,
            message: "filesystem provider is not installed".into(),
            retryable: false,
        }),
        trace_id: "trace-filesystem-unavailable".into(),
        descriptor_hash: filesystem_stable_hash(&snapshot),
    };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-filesystem-unavailable"));
    assert!(!serialized.contains("/Users/private/raw-host-path"));
    assert!(!serialized.contains("raw-file-bytes"));
    assert!(!serialized.contains("raw-secret-value"));
}
