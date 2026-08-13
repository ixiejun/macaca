use std::sync::Arc;

use macaca_proto::{
    communication_calendar_pack_definition, ApplicationImport, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};

use super::*;
use crate::loader::AppLoader;

#[test]
fn application_abi_projects_declared_calendar_scopes_and_commands() {
    let manifest = AppLoader::parse_manifest_yaml(
        r#"
name: calendar-abi-fixture
layer: L2Wasm
service_contract:
  optional_packs:
    - pack.communication.calendar.v1
  pack_permission_scopes:
    pack.communication.calendar.v1:
      - calendar.read.metadata
      - calendar.read.details
      - calendar.write
      - calendar.invite.respond
      - calendar.availability
      - calendar.reminder
      - calendar.conference
      - calendar.sync
      - calendar.watch
"#,
    )
    .unwrap();
    let mut calendar = communication_calendar_pack_definition();
    calendar.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(calendar);

    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.communication.calendar.v1")
        .expect("declared calendar pack must produce an ABI projection");

    for command in [
        "calendar.list_calendars",
        "calendar.query_events",
        "calendar.create_event",
        "calendar.respond_invite",
        "calendar.check_availability",
        "calendar.register_watch",
    ] {
        assert!(projection.callable_commands.contains(command));
    }
    assert!(descriptor
        .declaration
        .permissions
        .contains(&"calendar.write".into()));
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
}
