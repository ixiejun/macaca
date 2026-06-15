//! Package descriptor hooks for software drivers.
//!
//! Driver manifests already describe driver identity and capabilities.  This
//! adapter maps that metadata into Package Manifest v0 without
//! changing dynamic loading, built-in drivers, registry behavior, or sessions.

use macaca_proto::{
    CapabilityId, DeveloperId, PackageCapability, PackageDescriptor, PackageId, PackageManifest,
    PackageRuntime, PackageRuntimeKind, PackageType,
};

use crate::driver::DriverManifest;

/// Convert a driver manifest into a package descriptor.
pub fn driver_manifest_package_descriptor(manifest: &DriverManifest) -> PackageDescriptor {
    let mut package = PackageManifest::new(
        PackageId::new(format!("driver.{}", manifest.name)),
        PackageType::Driver,
        manifest.version.clone(),
        DeveloperId::new("local.driver"),
        PackageRuntime::new(PackageRuntimeKind::NativeAdapter, "1"),
    );
    for capability in &manifest.capabilities {
        package.provides.push(PackageCapability {
            id: CapabilityId::new(format!("driver.{}.{}", manifest.name, capability)),
            description: manifest.description.clone(),
        });
    }
    if let Some(trace_types) = &manifest.trace_event_types {
        package
            .metadata
            .insert("driver.trace_event_types".into(), trace_types.join(","));
    }
    package
        .metadata
        .insert("driver.type".into(), format!("{:?}", manifest.driver_type));
    PackageDescriptor::new(package)
}

#[cfg(test)]
mod tests {
    use macaca_proto::DriverId;

    use crate::driver::DriverType;

    use super::*;

    #[test]
    fn driver_manifest_maps_to_package_descriptor() {
        let manifest = DriverManifest {
            id: DriverId::new(),
            name: "sample".into(),
            version: "1.0.0".into(),
            driver_type: DriverType::CliSubprocess,
            description: "Sample driver".into(),
            capabilities: vec!["run".into()],
            trace_event_types: Some(vec!["tool_call".into()]),
        };

        let descriptor = driver_manifest_package_descriptor(&manifest);

        assert_eq!(descriptor.manifest.package_type, PackageType::Driver);
        assert_eq!(
            descriptor.manifest.runtime.kind,
            Some(PackageRuntimeKind::NativeAdapter)
        );
        assert!(descriptor
            .manifest
            .provides
            .iter()
            .any(|capability| capability.id.as_str().contains("driver.sample.run")));
    }
}
