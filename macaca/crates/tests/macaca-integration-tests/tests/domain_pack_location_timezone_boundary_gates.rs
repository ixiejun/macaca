//! Boundary gates for the location-timezone pack.

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
fn timezone_provider_is_runtime_host_only() {
    let workspace = root();
    for surface in [
        "crates/kernel",
        "crates/facade/macaca-sdk/src",
        "crates/shells",
        "crates/application/macaca-app/src",
    ] {
        let path = workspace.join(surface);
        let output = std::process::Command::new("rg")
            .args([
                "-n",
                "LocationTimezoneSystemServiceProvider|location_timezone_service_provider|tzdb_loader",
                path.to_string_lossy().as_ref(),
            ])
            .output()
            .expect("rg available");
        assert!(
            !output.status.success(),
            "timezone provider leaked into {surface}"
        );
    }
}

#[test]
fn timezone_commands_exclude_adjacent_location_and_schedule_owners() {
    let source = fs::read_to_string(
        root().join("crates/foundation/macaca-proto/src/domain_pack_contract/location_timezone.rs"),
    )
    .unwrap();
    let commands = source
        .lines()
        .filter_map(|line| line.split('\"').nth(1))
        .filter(|value| value.starts_with("timezone."))
        .collect::<Vec<_>>();
    for excluded in [
        "calendar",
        "schedule",
        "map_render",
        "geocode",
        "route",
        "device",
    ] {
        assert!(
            commands.iter().all(|command| !command.contains(excluded)),
            "timezone command unexpectedly owns {excluded}: {commands:?}"
        );
    }
}
