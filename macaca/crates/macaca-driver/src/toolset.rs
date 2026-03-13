//! `DriverToolSet` — aggregates tools from all drivers + standalone tools into a unified `ToolSet`.

use macaca_tools::{Tool, ToolSet};

/// A `ToolSet` that combines tools from all registered drivers
/// with any standalone tools.
pub struct DriverToolSet {
    tools: Vec<Box<dyn Tool>>,
}

impl DriverToolSet {
    /// Create from a list of driver tools and standalone tools.
    pub fn new(driver_tools: Vec<Box<dyn Tool>>, standalone_tools: Vec<Box<dyn Tool>>) -> Self {
        let mut tools = driver_tools;
        tools.extend(standalone_tools);
        Self { tools }
    }

    /// Create an empty toolset.
    pub fn empty() -> Self {
        Self { tools: vec![] }
    }
}

impl ToolSet for DriverToolSet {
    fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_toolset() {
        let ts = DriverToolSet::empty();
        assert!(ts.tools().is_empty());
    }

    #[test]
    fn combined_toolset() {
        let ts = DriverToolSet::new(vec![], vec![]);
        assert!(ts.tools().is_empty());
    }
}
