//! Package loader factory for Route C Package Manifest v0.
//!
//! Loader selection uses the Factory Method pattern.  The first production
//! loader is the YAML application metadata loader, which delegates to the
//! existing `AppLoader` compatibility path.  WASM packages are metadata-only
//! in Phase 04 and return a structured runtime-unavailable error for execution.

use std::path::Path;

use macaca_proto::{
    MacacaResult, PackageDescriptor, PackageGuardError, PackageRuntimeKind, PackageType,
};
use tracing::{info, warn};

use crate::package::load_yaml_app_package_descriptor;
use crate::runtime_guard::{
    package_requires_wasm_runtime, PackageGuardContext, PackageRuntimeGuard,
};

/// Result of selecting a package loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageLoaderKind {
    YamlApplication,
    WasmMetadataOnly,
}

/// Factory for package metadata loaders.
pub struct PackageLoaderFactory;

impl PackageLoaderFactory {
    /// Load package metadata from an existing YAML application manifest.
    pub fn load_yaml_application(path: impl AsRef<Path>) -> MacacaResult<PackageDescriptor> {
        info!(
            path = %path.as_ref().display(),
            "package loader selected YAML application compatibility path"
        );
        load_yaml_app_package_descriptor(path)
    }

    /// Select a loader for a descriptor without executing package code.
    pub fn select(descriptor: &PackageDescriptor) -> Result<PackageLoaderKind, PackageGuardError> {
        match (
            &descriptor.manifest.package_type,
            descriptor.manifest.runtime.kind.as_ref(),
        ) {
            (PackageType::Application, Some(PackageRuntimeKind::Yaml)) => {
                Ok(PackageLoaderKind::YamlApplication)
            }
            (_, Some(PackageRuntimeKind::WasmComponent)) => Ok(PackageLoaderKind::WasmMetadataOnly),
            (_, Some(kind)) => Err(PackageGuardError::UnsupportedRuntimeKind(kind.to_string())),
            (_, None) => Err(PackageGuardError::MissingRuntimeKind),
        }
    }

    /// Guard a descriptor and report whether execution is available.
    pub fn guard_for_metadata_load(
        descriptor: PackageDescriptor,
        context: &PackageGuardContext,
    ) -> PackageDescriptor {
        PackageRuntimeGuard::new().evaluate(descriptor, context)
    }

    /// Phase 04 execution entry intentionally rejects WASM execution.
    pub fn execute_if_available(descriptor: &PackageDescriptor) -> Result<(), PackageGuardError> {
        if package_requires_wasm_runtime(descriptor) {
            warn!(
                package_id = %descriptor.manifest.id,
                "WASM execution requested before runtime is installed"
            );
            return Err(PackageGuardError::RuntimeUnavailable(
                "WASM component execution belongs to the Application ABI runtime phase".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        DeveloperId, PackageDescriptor, PackageId, PackageManifest, PackageRuntime,
        PackageRuntimeKind, PackageType,
    };

    use super::*;

    fn first_example_app() -> std::path::PathBuf {
        let examples_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/apps");
        let mut paths = std::fs::read_dir(examples_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("app.yaml"))
            .filter(|path| path.exists())
            .collect::<Vec<_>>();
        paths.sort();
        paths.into_iter().next().unwrap()
    }

    #[test]
    fn yaml_loader_loads_example_metadata() {
        let descriptor = PackageLoaderFactory::load_yaml_application(first_example_app()).unwrap();

        assert_eq!(
            PackageLoaderFactory::select(&descriptor).unwrap(),
            PackageLoaderKind::YamlApplication
        );
    }

    #[test]
    fn wasm_execution_returns_runtime_unavailable() {
        let descriptor = PackageDescriptor::new(PackageManifest::new(
            PackageId::new("pkg.wasm"),
            PackageType::Application,
            "1.0.0",
            DeveloperId::new("dev.wasm"),
            PackageRuntime::new(PackageRuntimeKind::WasmComponent, "1"),
        ));

        let err = PackageLoaderFactory::execute_if_available(&descriptor).unwrap_err();

        assert!(matches!(err, PackageGuardError::RuntimeUnavailable(_)));
    }

    #[test]
    fn unsupported_runtime_kind_is_structured_error() {
        let descriptor = PackageDescriptor::new(PackageManifest::new(
            PackageId::new("pkg.remote"),
            PackageType::Application,
            "1.0.0",
            DeveloperId::new("dev.remote"),
            PackageRuntime::new(PackageRuntimeKind::RemoteService, "1"),
        ));

        let err = PackageLoaderFactory::select(&descriptor).unwrap_err();

        assert!(matches!(err, PackageGuardError::UnsupportedRuntimeKind(_)));
    }
}
