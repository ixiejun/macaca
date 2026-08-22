//! Application ABI admission proofs for descriptor-owned identity-profile scopes.

use super::ApplicationAbiAdapter;
use crate::loader::AppLoader;
use crate::YamlApplicationAbiAdapter;

#[test]
fn profile_permission_scope_is_rejected_when_not_descriptor_owned() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: profile-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.identity.profile.v1\n  pack_permission_scopes:\n    pack.identity.profile.v1:\n      - identity.profile.native\n",
    )
    .unwrap();
    let error = YamlApplicationAbiAdapter::new(manifest).load().unwrap_err();
    assert!(error
        .to_string()
        .contains("identity profile permission scope"));
}
