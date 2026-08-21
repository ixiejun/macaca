use std::collections::{BTreeMap, BTreeSet};

use super::*;

// These tests intentionally stop at descriptor and DTO compatibility. The
// session-state pack is a contract slice until a serviceized provider supplies
// durable mutation, checkpoint, restore, compaction, and export behavior.

#[test]
fn foundation_session_state_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_session_state_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_SESSION_STATE_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "session_state_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/session-state"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_SESSION_STATE_SERVICE_ID)
        .expect("session-state descriptor exposes command schemas");
    for command in FOUNDATION_SESSION_STATE_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "session_state.read",
        "session_state.write",
        "session_state.delete",
        "session_state.list",
        "session_state.checkpoint",
        "session_state.restore",
        "session_state.compact",
        "session_state.clear",
        "session_state.export",
        "session_state.inspect_recovery",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in [
        "embedded",
        "remote-session-store",
        "replay",
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
fn session_state_manifest_declarations_require_pack_and_bounded_retention() {
    let declaration = SessionStateManifestDeclaration {
        session: SessionStateSessionRef {
            session_id: "session-manifest".into(),
            task_id: Some("task-manifest".into()),
        },
        checkpoint_support_required: true,
        restore_support_required: true,
        compaction_support_required: false,
        retention: SessionStateRetentionPolicy {
            ttl_seconds: Some(60),
            max_checkpoints: 4,
            compact_after_revisions: 10,
        },
    };
    let mut contract = AppServiceContractConfig {
        session_state_declarations: vec![declaration.clone()],
        ..Default::default()
    };
    assert!(validate_session_state_declarations(&contract).is_err());
    contract
        .optional_packs
        .push(FOUNDATION_SESSION_STATE_PACK_ID.into());
    assert!(validate_session_state_declarations(&contract).is_ok());
    contract.session_state_declarations.push(declaration);
    assert!(validate_session_state_declarations(&contract).is_err());
}

#[test]
fn industrial_catalog_uses_foundation_session_state_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_SESSION_STATE_PACK_ID)
        .expect("industrial catalog includes foundation session-state");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_SESSION_STATE_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("embedded")
            .and_then(|descriptor| descriptor.metadata.get("checkpoints"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_session_state_command_dtos_are_serde_compatible() {
    let session = SessionStateSessionRef {
        session_id: "session-ref".into(),
        task_id: Some("task-ref".into()),
    };
    let key = SessionStateKeyRef {
        session: session.clone(),
        key: "draft.form".into(),
    };
    let value = SessionStateValueRef {
        value_ref: "artifact:bounded-state-value".into(),
        schema_id: Some("form.schema.v1".into()),
        secret_reference_required: false,
    };
    let revision = SessionStateRevision {
        revision_id: "rev-2".into(),
        previous_revision_id: Some("rev-1".into()),
    };
    let checkpoint = SessionStateCheckpointRef {
        checkpoint_id: "checkpoint-1".into(),
        session: session.clone(),
        revision_id: revision.revision_id.clone(),
    };
    let retention = SessionStateRetentionPolicy {
        ttl_seconds: Some(3_600),
        max_checkpoints: 8,
        compact_after_revisions: 50,
    };
    let plan = SessionStateRestorePlan {
        checkpoint: checkpoint.clone(),
        dry_run: true,
        cross_session_allowed: false,
    };

    let commands = vec![
        serde_json::to_value(SessionStateGetCommand { key: key.clone() }).unwrap(),
        serde_json::to_value(SessionStatePutCommand {
            key: key.clone(),
            value,
            expected_revision: Some(revision.clone()),
        })
        .unwrap(),
        serde_json::to_value(SessionStateDeleteCommand {
            key: key.clone(),
            expected_revision: Some(revision.clone()),
        })
        .unwrap(),
        serde_json::to_value(SessionStateMergePatchCommand {
            key,
            patch_ref: "artifact:bounded-merge-patch".into(),
            expected_revision: Some(revision.clone()),
        })
        .unwrap(),
        serde_json::to_value(SessionStateListKeysCommand {
            session: session.clone(),
            prefix: Some("draft.".into()),
            page_size: 50,
            cursor: None,
        })
        .unwrap(),
        serde_json::to_value(SessionStateCreateCheckpointCommand {
            session: session.clone(),
            retention,
        })
        .unwrap(),
        serde_json::to_value(SessionStateListCheckpointsCommand {
            session: session.clone(),
            cursor: None,
            page_size: 25,
        })
        .unwrap(),
        serde_json::to_value(SessionStateRestoreCheckpointCommand { plan }).unwrap(),
        serde_json::to_value(SessionStateCompareCheckpointCommand {
            left: checkpoint.clone(),
            right: checkpoint,
        })
        .unwrap(),
        serde_json::to_value(SessionStateCompactHistoryCommand {
            session: session.clone(),
            before_revision: revision,
            dry_run: true,
        })
        .unwrap(),
        serde_json::to_value(SessionStateClearSessionCommand {
            session: session.clone(),
            dry_run: true,
        })
        .unwrap(),
        serde_json::to_value(SessionStateExportRedactedCommand {
            session: session.clone(),
            redaction_level: "metadata_only".into(),
        })
        .unwrap(),
        serde_json::to_value(SessionStateInspectRecoveryCommand { session }).unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_SESSION_STATE_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_session_state_hashes_change_with_contract_content() {
    let request = SessionStateListKeysCommand {
        session: SessionStateSessionRef {
            session_id: "session-ref".into(),
            task_id: None,
        },
        prefix: Some("draft.".into()),
        page_size: 50,
        cursor: None,
    };
    let mut changed = request.clone();
    changed.page_size = 100;

    assert_eq!(
        session_state_stable_hash(&request),
        session_state_stable_hash(&request)
    );
    assert_ne!(
        session_state_stable_hash(&request),
        session_state_stable_hash(&changed)
    );

    let hashes = foundation_session_state_descriptor_hashes();
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
fn foundation_session_state_result_and_snapshot_dtos_are_bounded() {
    let redaction = SessionStateRedactionSummary {
        redacted_value_count: 3,
        redacted_secret_reference_count: 1,
    };
    let snapshot = SessionStateProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        revision_hashes: BTreeMap::from([("rev-1".into(), "revision-hash".into())]),
        checkpoint_hashes: BTreeMap::from([("checkpoint-1".into(), "checkpoint-hash".into())]),
        redaction_summary: redaction.clone(),
    };
    let unavailable: SessionStateResultEnvelope<SessionStateRecoveryMetadata> =
        SessionStateResultEnvelope {
            status: SessionStateResultStatus::Unavailable,
            data: None,
            error: Some(SessionStateError {
                code: SessionStateResultStatus::Unavailable,
                message: "session-state provider is not installed".into(),
                retryable: false,
            }),
            trace_id: "trace-session-state-unavailable".into(),
            descriptor_hash: session_state_stable_hash(&snapshot),
        };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-session-state-unavailable"));
    assert!(!serialized.contains("raw-form-value"));
    assert!(!serialized.contains("raw-secret-value"));
    assert!(!serialized.contains("provider-private-session-payload"));
    assert_eq!(snapshot.redaction_summary, redaction);
}
