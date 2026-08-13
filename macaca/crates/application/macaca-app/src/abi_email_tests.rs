use std::sync::Arc;

use macaca_proto::{
    communication_email_pack_definition, ApplicationImport, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};

use super::*;
use crate::loader::AppLoader;

#[test]
fn application_abi_projects_declared_email_commands_and_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml(
        r#"
name: email-abi-fixture
layer: L2Wasm
service_contract:
  optional_packs:
    - pack.communication.email.v1
  pack_permission_scopes:
    pack.communication.email.v1:
      - email.send
      - email.read
      - email.draft
      - email.attachment
      - email.mailbox.sync
      - email.mailbox.mutate
      - email.delivery.read
      - email.event.ingest
"#,
    )
    .unwrap();
    let mut email = communication_email_pack_definition();
    email.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(email);

    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.communication.email.v1")
        .expect("declared email pack must produce an ABI projection");

    for command in [
        "email.compose",
        "email.save_draft",
        "email.send",
        "email.sync_mailbox",
        "email.fetch_attachment",
        "email.ingest_event",
    ] {
        assert!(projection.callable_commands.contains(command));
    }
    assert!(descriptor
        .declaration
        .permissions
        .contains(&"email.event.ingest".into()));
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
}
