//! Notification diagnostics and developer-example redaction gate.
//!
//! This gate complements runtime provider tests by ensuring the SDK discovery
//! projection and the published pack guide retain only opaque secret references.

use std::fs;
use std::path::{Path, PathBuf};

use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    COMMUNICATION_NOTIFICATION_PACK_ID,
};
use macaca_sdk::{CatalogBackedDomainPackClient, DomainPackInspectCommand, SystemDomainPackClient};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(|path| {
            fs::read_to_string(path.join("Cargo.toml"))
                .ok()
                .filter(|content| content.contains("[workspace]"))
                .map(|_| path.to_path_buf())
        })
        .expect("workspace root")
}

#[tokio::test]
async fn notification_sdk_diagnostics_and_examples_exclude_raw_secret_markers() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let inspection = client
        .inspect_pack(&DomainPackInspectCommand::new(COMMUNICATION_NOTIFICATION_PACK_ID).unwrap())
        .await
        .unwrap();
    let guide = fs::read_to_string(
        workspace_root().join("docs/developer-packs/communication/notification.md"),
    )
    .unwrap();
    let observable = format!("{:?}\n{guide}", inspection.pack);
    for raw_marker in [
        "raw-token-value",
        "https://push.example/private",
        "private-key-material",
        "credential-value",
        "provider-payload-value",
        "private-message-content",
    ] {
        assert!(!observable.contains(raw_marker));
    }
    assert!(guide.contains("secret:push-subscription"));
}
