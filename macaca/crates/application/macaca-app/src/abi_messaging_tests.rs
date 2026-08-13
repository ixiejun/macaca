use std::sync::Arc;

use macaca_proto::{
    communication_messaging_pack_definition, ApplicationImport, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};

use super::*;
use crate::loader::AppLoader;

#[test]
fn application_abi_projects_declared_messaging_commands_and_capabilities() {
    let manifest = AppLoader::parse_manifest_yaml(
        r#"
name: messaging-abi-fixture
layer: L2Wasm
service_contract:
  optional_packs:
    - pack.communication.messaging.v1
  pack_permission_scopes:
    pack.communication.messaging.v1:
      - messaging.send
      - messaging.read
      - messaging.conversation.manage
      - messaging.edit
      - messaging.delete
      - messaging.reaction
      - messaging.attachment
      - messaging.read_receipt
      - messaging.delivery.read
      - messaging.typing
      - messaging.event.ingest
"#,
    )
    .unwrap();
    let mut messaging = communication_messaging_pack_definition();
    messaging.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(messaging);

    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.communication.messaging.v1")
        .expect("declared messaging pack must produce an ABI projection");

    for command in [
        "messaging.find_conversation",
        "messaging.create_conversation",
        "messaging.send_message",
        "messaging.reply_message",
        "messaging.add_reaction",
        "messaging.ingest_event",
    ] {
        assert!(projection.callable_commands.contains(command));
    }
    assert!(descriptor
        .declaration
        .permissions
        .contains(&"messaging.event.ingest".into()));
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
}
