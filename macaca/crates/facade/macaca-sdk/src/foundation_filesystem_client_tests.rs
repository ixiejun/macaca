//! Tests proving filesystem SDK helpers remain a provider-neutral façade.

use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    FilesystemAdmissionFailure, TraceContext, FOUNDATION_FILESYSTEM_COMMANDS,
    FOUNDATION_FILESYSTEM_PACK_ID, FOUNDATION_FILESYSTEM_SERVICE_ID,
};

use super::*;
use crate::domain_pack_client::SystemDomainPackClient;
use crate::{CatalogBackedDomainPackClient, DomainPackResolveCommand};

async fn resolved() -> crate::DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_filesystem_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_FILESYSTEM_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn filesystem_helpers_build_all_declared_traced_service_calls() {
    let builders = vec![
        filesystem_open_handle_command,
        filesystem_close_handle_command,
        filesystem_read_file_command,
        filesystem_write_file_command,
        filesystem_append_file_command,
        filesystem_list_directory_command,
        filesystem_stat_path_command,
        filesystem_create_directory_command,
        filesystem_copy_path_command,
        filesystem_move_path_command,
        filesystem_delete_path_command,
        filesystem_create_temp_command,
        filesystem_watch_path_command,
        filesystem_snapshot_tree_command,
        filesystem_restore_snapshot_command,
    ];
    let resolved = resolved().await;
    for (command_name, builder) in FOUNDATION_FILESYSTEM_COMMANDS.iter().zip(builders) {
        let command = builder(
            serde_json::json!({"path":{"relative_path":"document.txt"}}),
            TraceContext::new(format!("trace-{command_name}")),
        )
        .build(&resolved)
        .unwrap();
        assert_eq!(command.service_id, FOUNDATION_FILESYSTEM_SERVICE_ID);
        assert_eq!(&command.command_name, command_name);
        assert!(command.trace.is_some());
    }
}

#[tokio::test]
async fn rejected_filesystem_preflight_cannot_create_a_service_call() {
    let outcome = filesystem_delete_path_command(
        serde_json::json!({"path":{"relative_path":"document.txt"}}),
        TraceContext::new("trace-filesystem-preflight-rejected"),
    )
    .build_after_preflight(
        &resolved().await,
        Err(FilesystemAdmissionFailure::ApprovalRequired),
    )
    .unwrap();
    assert_eq!(
        outcome,
        FilesystemDomainPackCommandBuildOutcome::Rejected(
            FilesystemAdmissionFailure::ApprovalRequired
        )
    );
}
