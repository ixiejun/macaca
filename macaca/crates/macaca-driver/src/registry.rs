//! `DriverRegistry` — discovers, loads, and manages installed software drivers.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use macaca_proto::{MacacaError, MacacaResult, DriverId};
use macaca_tools::Tool;

use crate::driver::{DriverManifest, SoftwareDriver};

/// Registry managing all installed software drivers.
///
/// Thread-safe: can be shared across agents and tasks.
pub struct DriverRegistry {
    drivers: Arc<RwLock<HashMap<DriverId, Box<dyn SoftwareDriver>>>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self {
            drivers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a driver. Returns its ID.
    pub async fn register(&self, driver: Box<dyn SoftwareDriver>) -> DriverId {
        let id = driver.manifest().id;
        self.drivers.write().await.insert(id, driver);
        id
    }

    /// Unregister a driver by ID.
    pub async fn unregister(&self, id: &DriverId) -> MacacaResult<()> {
        self.drivers
            .write()
            .await
            .remove(id)
            .ok_or_else(|| MacacaError::NotFound(format!("Driver {} not found", id)))?;
        Ok(())
    }

    /// List all registered driver manifests.
    pub async fn list_drivers(&self) -> Vec<DriverManifest> {
        self.drivers
            .read()
            .await
            .values()
            .map(|d| d.manifest().clone())
            .collect()
    }

    /// Get the number of registered drivers.
    pub async fn count(&self) -> usize {
        self.drivers.read().await.len()
    }

    /// Collect all tools from all registered drivers.
    pub async fn aggregate_tools(&self) -> Vec<Box<dyn Tool>> {
        let guard = self.drivers.read().await;
        let mut all_tools = Vec::new();
        for driver in guard.values() {
            all_tools.extend(driver.tools());
        }
        all_tools
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{DriverType, SoftwareDriver};
    use async_trait::async_trait;

    struct DummyDriver {
        manifest: DriverManifest,
    }

    impl DummyDriver {
        fn new(name: &str) -> Self {
            Self {
                manifest: DriverManifest {
                    id: DriverId::new(),
                    name: name.into(),
                    version: "0.1.0".into(),
                    driver_type: DriverType::CliSubprocess,
                    description: format!("Dummy {name} driver"),
                    capabilities: vec![],
                },
            }
        }
    }

    #[async_trait]
    impl SoftwareDriver for DummyDriver {
        fn manifest(&self) -> &DriverManifest {
            &self.manifest
        }
        async fn initialize(&mut self) -> MacacaResult<()> {
            Ok(())
        }
        fn tools(&self) -> Vec<Box<dyn Tool>> {
            vec![]
        }
        async fn health_check(&self) -> MacacaResult<bool> {
            Ok(true)
        }
        async fn shutdown(&mut self) -> MacacaResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn register_and_list() {
        let registry = DriverRegistry::new();
        let driver = DummyDriver::new("test");
        let id = registry.register(Box::new(driver)).await;

        let drivers = registry.list_drivers().await;
        assert_eq!(drivers.len(), 1);
        assert_eq!(drivers[0].id, id);
        assert_eq!(drivers[0].name, "test");
    }

    #[tokio::test]
    async fn unregister() {
        let registry = DriverRegistry::new();
        let driver = DummyDriver::new("removable");
        let id = registry.register(Box::new(driver)).await;
        assert_eq!(registry.count().await, 1);

        registry.unregister(&id).await.unwrap();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn unregister_missing_returns_error() {
        let registry = DriverRegistry::new();
        let fake_id = DriverId::new();
        assert!(registry.unregister(&fake_id).await.is_err());
    }

    #[tokio::test]
    async fn aggregate_tools_empty() {
        let registry = DriverRegistry::new();
        registry.register(Box::new(DummyDriver::new("a"))).await;
        let tools = registry.aggregate_tools().await;
        assert!(tools.is_empty()); // DummyDriver has no tools
    }
}
