//! `DriverToolSet` — compatibility wrapper over `macaca_tools::CompositeToolSet`.

use macaca_tools::{CompositeToolSet, Tool, ToolSet};

/// A `ToolSet` that combines tools from all registered drivers
/// with any standalone tools.
pub struct DriverToolSet {
    inner: CompositeToolSet,
}

impl DriverToolSet {
    /// Create from a list of driver tools and standalone tools.
    #[deprecated(note = "use macaca_tools::CompositeToolSet::from_groups()")]
    pub fn new(driver_tools: Vec<Box<dyn Tool>>, standalone_tools: Vec<Box<dyn Tool>>) -> Self {
        Self {
            inner: CompositeToolSet::from_groups(vec![driver_tools, standalone_tools]),
        }
    }

    /// Create an empty toolset.
    #[deprecated(note = "use macaca_tools::CompositeToolSet::empty()")]
    pub fn empty() -> Self {
        Self {
            inner: CompositeToolSet::empty(),
        }
    }

    pub fn from_composite(inner: CompositeToolSet) -> Self {
        Self { inner }
    }
}

impl ToolSet for DriverToolSet {
    #[allow(deprecated)]
    fn tools(&self) -> &[Box<dyn Tool>] {
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
