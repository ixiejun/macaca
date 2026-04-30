//! Driver Loader - Discovers and loads external driver plugins from a directory.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::{error, info, warn};

use crate::driver::SoftwareDriver;
use crate::dynamic_driver::DynamicDriver;
use crate::plugin_abi::DRIVER_ABI_VERSION;
use macaca_proto::MacacaResult;

/// Manifest parsed from driver.toml
#[derive(Debug, Deserialize)]
pub struct DriverManifestToml {
    pub driver: DriverSection,
    /// Arbitrary config passed to the driver as JSON
    pub config: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct DriverSection {
    pub name: String,
    pub version: String,
    pub description: String,
    pub library: String,
    #[serde(default = "default_min_abi")]
    pub min_abi_version: u32,
}

fn default_min_abi() -> u32 {
    1
}

/// Result of attempting to load a single driver
pub struct DriverLoadResult {
    pub name: String,
    pub path: PathBuf,
    pub result: Result<Box<dyn SoftwareDriver>, String>,
}

/// Discovers and loads external driver plugins from a directory.
///
/// Each driver is a subdirectory containing a `driver.toml` manifest
/// and a shared library (`.dylib` / `.so`).
pub struct DriverLoader {
    drivers_dir: PathBuf,
}

impl DriverLoader {
    pub fn new(drivers_dir: impl Into<PathBuf>) -> Self {
        Self {
            drivers_dir: drivers_dir.into(),
        }
    }

    /// Returns the drivers directory path.
    pub fn drivers_dir(&self) -> &Path {
        &self.drivers_dir
    }

    /// Scan the drivers directory and return all discovered driver manifests.
    pub fn scan(&self) -> Vec<(PathBuf, DriverManifestToml)> {
        let mut results = Vec::new();

        let entries = match std::fs::read_dir(&self.drivers_dir) {
            Ok(e) => e,
            Err(e) => {
                warn!(dir = %self.drivers_dir.display(), error = %e, "Failed to read drivers directory");
                return results;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "Failed to read directory entry");
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("driver.toml");
            if !manifest_path.exists() {
                warn!(dir = %path.display(), "Driver directory missing driver.toml, skipping");
                continue;
            }

            match std::fs::read_to_string(&manifest_path) {
                Ok(content) => match toml::from_str::<DriverManifestToml>(&content) {
                    Ok(manifest) => {
                        info!(
                            name = %manifest.driver.name,
                            version = %manifest.driver.version,
                            "Discovered driver"
                        );
                        results.push((path, manifest));
                    }
                    Err(e) => {
                        warn!(path = %manifest_path.display(), error = %e, "Failed to parse driver.toml");
                    }
                },
                Err(e) => {
                    warn!(path = %manifest_path.display(), error = %e, "Failed to read driver.toml");
                }
            }
        }

        results
    }

    /// Load a single driver from its directory and manifest.
    pub fn load_driver(
        &self,
        driver_dir: &Path,
        manifest: &DriverManifestToml,
    ) -> MacacaResult<Box<dyn SoftwareDriver>> {
        // Check ABI version compatibility
        if manifest.driver.min_abi_version > DRIVER_ABI_VERSION {
            return Err(macaca_proto::MacacaError::Driver(format!(
                "Driver '{}' requires ABI version {} but host supports version {}",
                manifest.driver.name, manifest.driver.min_abi_version, DRIVER_ABI_VERSION
            )));
        }

        // Resolve library path
        let library_path = driver_dir.join(&manifest.driver.library);
        if !library_path.exists() {
            return Err(macaca_proto::MacacaError::Driver(format!(
                "Driver library not found: {}",
                library_path.display()
            )));
        }

        // Convert config to JSON string
        let config_json = match &manifest.config {
            Some(config) => serde_json::to_string(config).unwrap_or_else(|_| "{}".to_string()),
            None => "{}".to_string(),
        };

        // Load the dynamic driver
        info!(
            name = %manifest.driver.name,
            library = %library_path.display(),
            "Loading driver"
        );

        let driver = DynamicDriver::load(&library_path, &config_json)?;

        info!(
            name = %manifest.driver.name,
            version = %manifest.driver.version,
            "Driver loaded successfully"
        );

        Ok(Box::new(driver))
    }

    /// Scan and load all drivers from the directory.
    ///
    /// Returns results for every discovered driver, including failures.
    pub fn load_all(&self) -> Vec<DriverLoadResult> {
        info!(dir = %self.drivers_dir.display(), "Loading all drivers");

        if !self.drivers_dir.exists() {
            info!(dir = %self.drivers_dir.display(), "Drivers directory does not exist, creating it");
            if let Err(e) = std::fs::create_dir_all(&self.drivers_dir) {
                error!(dir = %self.drivers_dir.display(), error = %e, "Failed to create drivers directory");
            }
            return Vec::new();
        }

        let discovered = self.scan();
        let mut results = Vec::new();

        for (path, manifest) in discovered {
            let name = manifest.driver.name.clone();
            let result = self.load_driver(&path, &manifest);
            results.push(DriverLoadResult {
                name,
                path: path.clone(),
                result: result.map_err(|e| e.to_string()),
            });
        }

        let (success, failed): (Vec<_>, Vec<_>) = results.iter().partition(|r| r.result.is_ok());

        info!(
            total = results.len(),
            success = success.len(),
            failed = failed.len(),
            "Driver loading complete"
        );

        for f in &failed {
            error!(
                name = %f.name,
                path = %f.path.display(),
                error = ?f.result.as_ref().err(),
                "Failed to load driver"
            );
        }

        results
    }
}
