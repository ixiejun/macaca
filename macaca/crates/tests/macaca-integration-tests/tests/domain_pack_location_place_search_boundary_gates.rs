//! Boundary and scope gates for the location place-search pack.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
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

#[test]
fn place_search_provider_is_not_imported_by_os_surfaces() {
    let workspace = root();
    for surface in [
        "crates/kernel",
        "crates/facade/macaca-sdk/src",
        "crates/shells",
        "crates/application/macaca-app/src",
    ] {
        let output = std::process::Command::new("rg")
            .args([
                "-n",
                "LocationPlaceSearchSystemServiceProvider|location_place_search_service_provider",
                &workspace.join(surface).to_string_lossy(),
            ])
            .output()
            .expect("rg available");
        assert!(
            !output.status.success(),
            "place-search concrete provider leaked into {surface}"
        );
    }
}

#[test]
fn place_search_commands_exclude_adjacent_location_owners() {
    let source =
        fs::read_to_string(root().join(
            "crates/foundation/macaca-proto/src/domain_pack_contract/location_place_search.rs",
        ))
        .unwrap();
    let commands = source
        .lines()
        .filter_map(|line| line.split('\"').nth(1))
        .filter(|value| value.starts_with("place_search."))
        .collect::<Vec<_>>();
    for excluded in ["map", "geocode", "route", "timezone", "device", "booking"] {
        assert!(commands.iter().all(|command| !command.contains(excluded)));
    }
}
