//! Tests for SDK session-state helper routing and dry-run safety.

use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    SessionStateCheckpointRef, SessionStateKeyRef, SessionStateRetentionPolicy,
    SessionStateRevision, SessionStateSessionRef, SessionStateValueRef, TraceContext,
    FOUNDATION_SESSION_STATE_PACK_ID, FOUNDATION_SESSION_STATE_SERVICE_ID,
};

use super::*;
use crate::domain_pack_client::{EmptySystemDomainPackClient, SystemDomainPackClient};
use crate::{CatalogBackedDomainPackClient, DomainPackResolveCommand};

async fn resolved() -> DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_session_state_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_SESSION_STATE_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

fn session() -> SessionStateSessionRef {
    SessionStateSessionRef {
        session_id: "session-sdk".into(),
        task_id: Some("task-sdk".into()),
    }
}

fn key() -> SessionStateKeyRef {
    SessionStateKeyRef {
        session: session(),
        key: "form.field".into(),
    }
}

#[tokio::test]
async fn helpers_build_only_canonical_traced_service_calls() {
    let put = session_state_put_command(
        key(),
        SessionStateValueRef {
            value_ref: "artifact:state".into(),
            schema_id: Some("form.v1".into()),
            secret_reference_required: false,
        },
        None,
        TraceContext::new("session-sdk-put"),
    )
    .unwrap()
    .build(&resolved().await)
    .unwrap();
    assert_eq!(put.service_id, FOUNDATION_SESSION_STATE_SERVICE_ID);
    assert_eq!(put.command_name, "session_state.put");
    assert_eq!(put.trace.unwrap().trace_id, "session-sdk-put");
    assert!(!put.payload.to_string().contains("provider"));

    let get = session_state_get_command(key(), TraceContext::new("session-sdk-get"))
        .unwrap()
        .build(&resolved().await)
        .unwrap();
    assert_eq!(get.command_name, "session_state.get");
}

#[tokio::test]
async fn destructive_helpers_remain_dry_run_and_diagnostics_are_scoped() {
    let checkpoint = SessionStateCheckpointRef {
        checkpoint_id: "checkpoint:opaque".into(),
        session: session(),
        revision_id: "revision:opaque".into(),
    };
    let restore = session_state_restore_dry_run_command(checkpoint, TraceContext::new("restore"))
        .unwrap()
        .build(&resolved().await)
        .unwrap();
    assert_eq!(restore.payload["plan"]["dry_run"], true);

    let compact = session_state_compact_dry_run_command(
        session(),
        SessionStateRevision {
            revision_id: "revision:opaque".into(),
            previous_revision_id: None,
        },
        TraceContext::new("compact"),
    )
    .unwrap()
    .build(&resolved().await)
    .unwrap();
    assert_eq!(compact.payload["dry_run"], true);
    assert!(session_state_validate_page_size(0).is_err());
    assert!(session_state_validate_page_size(100).is_ok());

    let unavailable = EmptySystemDomainPackClient
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_SESSION_STATE_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap();
    assert_eq!(session_state_unavailable_diagnostics(&unavailable).len(), 1);
}

#[test]
fn checkpoint_helpers_preserve_bounded_retention_intent() {
    let builder = session_state_create_checkpoint_command(
        session(),
        SessionStateRetentionPolicy {
            ttl_seconds: Some(60),
            max_checkpoints: 4,
            compact_after_revisions: 10,
        },
        TraceContext::new("checkpoint"),
    )
    .unwrap();
    assert_eq!(builder.command_name, "session_state.create_checkpoint");
}
