//! Driver runtime facade.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use macaca_tools::Tool;

use crate::driver::DriverManifest;
use crate::load_command::{
    DriverLoadCommand, DriverLoadEntry, DriverLoadReport, DriverLoadStatus,
};
use crate::loader::DriverLoader;
use crate::registry::DriverRegistry;

/// Driver inventory item returned by the runtime facade.
#[derive(Debug, Clone)]
pub struct DriverInventoryItem {
    pub manifest: DriverManifest,
    pub tool_count: usize,
}

/// Facade over driver discovery, registry mutation, inventory, and tool collection.
pub struct DriverRuntime {
    drivers_dir: PathBuf,
    registry: Arc<DriverRegistry>,
}

impl DriverRuntime {
    pub fn new(drivers_dir: impl Into<PathBuf>, registry: Arc<DriverRegistry>) -> Self {
        Self {
            drivers_dir: drivers_dir.into(),
            registry,
        }
    }

    pub fn drivers_dir(&self) -> &Path {
        &self.drivers_dir
    }

    pub fn registry(&self) -> Arc<DriverRegistry> {
        Arc::clone(&self.registry)
    }

    pub async fn load_all(&self) -> DriverLoadReport {
        self.load_with_command(DriverLoadCommand::LoadAll, false)
            .await
    }

    pub async fn reload(&self) -> DriverLoadReport {
        self.load_with_command(DriverLoadCommand::Reload, true).await
    }

    async fn load_with_command(
        &self,
        command: DriverLoadCommand,
        clear_first: bool,
    ) -> DriverLoadReport {
        if clear_first {
            self.registry.clear().await;
        }

        let loader = DriverLoader::new(&self.drivers_dir);
        let results = loader.load_all_internal();
        let mut entries = Vec::new();
        let mut loaded = 0usize;
        let mut failed = 0usize;

        for result in results {
            match result.result {
                Ok(driver) => {
                    let tool_count = crate::SoftwareDriver::tools(driver.as_ref()).len();
                    self.registry.register(driver).await;
                    loaded += 1;
                    entries.push(DriverLoadEntry {
                        name: result.name,
                        path: result.path,
                        status: DriverLoadStatus::Loaded,
                        tool_count: Some(tool_count),
                        error: None,
                    });
                }
                Err(error) => {
                    failed += 1;
                    entries.push(DriverLoadEntry {
                        name: result.name,
                        path: result.path,
                        status: DriverLoadStatus::Failed,
                        tool_count: None,
                        error: Some(error),
                    });
                }
            }
        }

        DriverLoadReport {
            command,
            loaded,
            failed,
            entries,
        }
    }

    pub async fn list_inventory(&self) -> Vec<DriverInventoryItem> {
        self.registry
            .list_drivers_with_tools()
            .await
            .into_iter()
            .map(|(manifest, tool_count)| DriverInventoryItem {
                manifest,
                tool_count,
            })
            .collect()
    }

    pub async fn collect_tools(&self) -> Vec<Box<dyn Tool>> {
        self.registry.collect_tools().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_uses_shared_registry() {
        let registry = Arc::new(DriverRegistry::new());
        let runtime = DriverRuntime::new("/tmp/drivers", Arc::clone(&registry));

        assert!(Arc::ptr_eq(&registry, &runtime.registry()));
        assert_eq!(runtime.drivers_dir(), Path::new("/tmp/drivers"));
    }
}
