//! Organization and tenant providers remain runtime-host implementation details.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];
const FORBIDDEN: &[&str] = &[
    "IdentityOrganizationSystemServiceProvider",
    "identity_organization_service_provider",
    "IdentityProfileSystemServiceProvider",
    "identity_profile_service_provider",
    "IdentityAuthHandoffSystemServiceProvider",
    "identity_auth_handoff_service_provider",
    "IdentityTenantSystemServiceProvider",
    "identity_tenant_service_provider",
    "Auth0",
    "Clerk",
    "WorkOS",
    "Okta",
    "Microsoft Graph",
    "Google",
    "SCIM",
    "GitHub",
    "directory-sync",
    "invitation-delivery",
    "AWS",
    "Azure",
    "Kubernetes",
    "OIDC",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(|path| {
            fs::read_to_string(path.join("Cargo.toml"))
                .ok()
                .filter(|text| text.contains("[workspace]"))
                .map(|_| path.to_path_buf())
        })
        .expect("workspace root")
}

fn collect_sources(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

#[test]
fn identity_governance_boundaries_do_not_import_or_construct_concrete_providers() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for surface in SURFACES {
        let mut sources = Vec::new();
        collect_sources(&root.join(surface), &mut sources);
        for source in sources {
            for (line_number, line) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if !line.trim_start().starts_with("//") {
                    for token in FORBIDDEN {
                        if line.contains(token) {
                            violations.push(format!(
                                "{}:{}:{token}",
                                source.strip_prefix(&root).unwrap().display(),
                                line_number + 1
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "identity-governance provider boundary violations:\n{}",
        violations.join("\n")
    );
}
