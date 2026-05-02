//! Driver factory primitives.

use std::path::PathBuf;

use macaca_proto::MacacaResult;

use crate::driver::SoftwareDriver;
use crate::dynamic_driver::DynamicDriver;

/// Creation context passed to driver factories.
#[derive(Debug, Clone)]
pub struct DriverCreateContext {
    pub config_json: String,
}

impl DriverCreateContext {
    pub fn new(config_json: impl Into<String>) -> Self {
        Self {
            config_json: config_json.into(),
        }
    }
}

/// Factory abstraction for constructing software drivers.
pub trait DriverFactory: Send + Sync {
    fn driver_name(&self) -> &str;

    fn create(&self, ctx: DriverCreateContext) -> MacacaResult<Box<dyn SoftwareDriver>>;
}

/// Factory for dynamic C-ABI drivers.
pub struct DynamicDriverFactory {
    name: String,
    library_path: PathBuf,
}

impl DynamicDriverFactory {
    pub fn new(name: impl Into<String>, library_path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            library_path: library_path.into(),
        }
    }

    pub fn library_path(&self) -> &PathBuf {
        &self.library_path
    }
}

impl DriverFactory for DynamicDriverFactory {
    fn driver_name(&self) -> &str {
        &self.name
    }

    fn create(&self, ctx: DriverCreateContext) -> MacacaResult<Box<dyn SoftwareDriver>> {
        let driver = DynamicDriver::load_dynamic(&self.library_path, &ctx.config_json)?;
        Ok(Box::new(driver))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_context_stores_config_json() {
        let ctx = DriverCreateContext::new(r#"{"mode":"test"}"#);
        assert_eq!(ctx.config_json, r#"{"mode":"test"}"#);
    }

    #[test]
    fn dynamic_factory_exposes_name_and_library_path() {
        let factory = DynamicDriverFactory::new("driver-a", "/tmp/driver.dylib");
        assert_eq!(factory.driver_name(), "driver-a");
        assert_eq!(factory.library_path(), &PathBuf::from("/tmp/driver.dylib"));
    }
}
