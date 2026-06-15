//! Fluent builder for assembling [`super::MacacaConfig`] in tests and SDK helpers.

use super::{DriversConfig, KernelConfig, LlmConfig, MacacaConfig, WorkspaceConfig};

/// Builder that starts from [`MacacaConfig::default`] and overrides selected sections.
pub struct MacacaConfigBuilder {
    inner: MacacaConfig,
}

impl MacacaConfigBuilder {
    pub fn new() -> Self {
        Self {
            inner: MacacaConfig::default(),
        }
    }

    pub fn kernel(mut self, kernel: KernelConfig) -> Self {
        self.inner.kernel = kernel;
        self
    }

    pub fn llm(mut self, llm: LlmConfig) -> Self {
        self.inner.llm = llm;
        self
    }

    pub fn workspace(mut self, workspace: WorkspaceConfig) -> Self {
        self.inner.workspace = workspace;
        self
    }

    pub fn drivers(mut self, drivers: DriversConfig) -> Self {
        self.inner.drivers = drivers;
        self
    }

    pub fn build(self) -> MacacaConfig {
        self.inner
    }
}

impl Default for MacacaConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
