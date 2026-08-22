//! Identity-profile commands must remain outside neighboring identity owners.

use std::fs;
use std::path::PathBuf;

#[test]
fn profile_provider_does_not_own_account_auth_or_application_preferences() {
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_root
        .ancestors()
        .find(|path| {
            fs::read_to_string(path.join("Cargo.toml"))
                .ok()
                .is_some_and(|contents| contents.contains("[workspace]"))
        })
        .expect("workspace root");
    let source = fs::read_to_string(
        root.join("crates/runtime/macaca-runtime-host/src/identity_profile_service_provider.rs"),
    )
    .unwrap();
    for forbidden in [
        "account_lifecycle",
        "token_exchange",
        "credential_storage",
        "mfa_execution",
        "organization_membership",
        "tenant_policy",
        "media_processing",
        "application_preference_workflow",
    ] {
        assert!(
            !source.contains(forbidden),
            "profile boundary contains {forbidden}"
        );
    }
}
