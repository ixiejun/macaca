//! `DriverToolSet` — driver-owned catalog wrapper over `macaca_tools::CompositeToolSet`.

use macaca_tools::{CompositeToolSet, Tool, ToolCatalog};

/// Driver-owned tool catalog that combines registered driver tools with any
/// standalone tools.
pub struct DriverToolSet {
    inner: CompositeToolSet,
}

impl DriverToolSet {
    /// Create from a list of driver tools and standalone tools.
    pub fn from_groups(
        driver_tools: Vec<Box<dyn Tool>>,
        standalone_tools: Vec<Box<dyn Tool>>,
    ) -> Self {
        Self {
            inner: CompositeToolSet::from_groups(vec![driver_tools, standalone_tools]),
        }
    }

    /// Create an empty toolset.
    pub fn empty() -> Self {
        Self {
            inner: CompositeToolSet::empty(),
        }
    }

    pub fn from_composite(inner: CompositeToolSet) -> Self {
        Self { inner }
    }
}

impl ToolCatalog for DriverToolSet {
    fn all_tools(&self) -> &[Box<dyn Tool>] {
        macaca_tools::ToolCatalog::all_tools(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_toolset() {
        let ts = DriverToolSet::from_composite(CompositeToolSet::empty());
        assert!(macaca_tools::ToolCatalog::all_tools(&ts).is_empty());
    }

    #[test]
    fn combined_toolset() {
        let ts = DriverToolSet::from_composite(CompositeToolSet::from_groups(vec![vec![], vec![]]));
        assert!(macaca_tools::ToolCatalog::all_tools(&ts).is_empty());
    }
}
