//! `FileSystemDriver` — wraps `FileReadTool` + `FileWriteTool` as a `SoftwareDriver`.
//!
//! Provides filesystem operations as a pluggable driver.

use async_trait::async_trait;

use macaca_proto::{DriverId, MacacaResult};
use macaca_tools::builtin::{FileReadTool, FileWriteTool};
use macaca_tools::Tool;

use crate::driver::{DriverManifest, DriverType, SoftwareDriver};

/// A built-in driver that exposes file read/write operations.
pub struct FileSystemDriver {
    manifest: DriverManifest,
}

impl FileSystemDriver {
    /// Create a new FileSystemDriver.
    pub fn new() -> Self {
        Self {
            manifest: DriverManifest {
                id: DriverId::new(),
                name: "filesystem".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                driver_type: DriverType::FileIpc,
                description: "Read and write files on the local filesystem".into(),
                capabilities: vec!["file_read".into(), "file_write".into()],
                trace_event_types: None,
            },
        }
    }
}

impl Default for FileSystemDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SoftwareDriver for FileSystemDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        // Filesystem is always available — no initialization needed.
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(FileReadTool), Box::new(FileWriteTool)]
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        Ok(true)
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_driver_manifest() {
        let driver = FileSystemDriver::new();
        let m = driver.manifest();
        assert_eq!(m.name, "filesystem");
        assert_eq!(m.driver_type, DriverType::FileIpc);
        assert!(m.capabilities.contains(&"file_read".into()));
        assert!(m.capabilities.contains(&"file_write".into()));
    }

    #[test]
    fn filesystem_driver_tools() {
        let driver = FileSystemDriver::new();
        let tools = driver.tools();
        assert_eq!(tools.len(), 2);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
    }

    #[tokio::test]
    async fn filesystem_driver_lifecycle() {
        let mut driver = FileSystemDriver::new();
        driver.initialize().await.unwrap();
        assert!(driver.health_check().await.unwrap());
        driver.shutdown().await.unwrap();
    }
}
