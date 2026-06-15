//! Composite capability tree for application-level capability tracking.
//!
//! Applies the **Composite** pattern: capability groups preserve provenance
//! (`Manifest`, `Skill`, `Driver`, etc.) while `flatten` returns the flat list
//! consumed by admission and projection code paths.

use super::agent_config::{AgentSource, CapabilityRef};

/// Internal source of application capability information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCapabilitySource {
    Manifest,
    Skill,
    Driver,
    ToolPolicy,
    Provider,
}

/// Internal composite capability node for application-level capability tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCapabilityNode {
    Leaf(CapabilityRef),
    Group {
        source: AppCapabilitySource,
        children: Vec<AppCapabilityNode>,
    },
}

impl AppCapabilityNode {
    /// Depth-first flatten helper that collects leaf capability references.
    fn flatten_into<'a>(&'a self, output: &mut Vec<&'a CapabilityRef>) {
        match self {
            Self::Leaf(capability) => output.push(capability),
            Self::Group { children, .. } => {
                for child in children {
                    child.flatten_into(output);
                }
            }
        }
    }
}

/// Composite capability set for applications.
///
/// Callers can flatten back to a capability list while preserving source
/// information internally for structured admission diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppCapabilitySet {
    root: Vec<AppCapabilityNode>,
}

impl AppCapabilitySet {
    /// Construct a manifest-backed capability set from inline agent definitions.
    pub fn from_manifest_agents(agents: &[AgentSource]) -> Self {
        let mut root = Vec::new();
        for source in agents {
            if let AgentSource::Inline(inline) = source {
                if inline.capabilities.is_empty() {
                    continue;
                }
                root.push(AppCapabilityNode::Group {
                    source: AppCapabilitySource::Manifest,
                    children: inline
                        .capabilities
                        .iter()
                        .cloned()
                        .map(AppCapabilityNode::Leaf)
                        .collect(),
                });
            }
        }
        Self { root }
    }

    /// Add a capability group from a specific source.
    pub fn push_group(&mut self, source: AppCapabilitySource, capabilities: Vec<CapabilityRef>) {
        if capabilities.is_empty() {
            return;
        }
        self.root.push(AppCapabilityNode::Group {
            source,
            children: capabilities
                .into_iter()
                .map(AppCapabilityNode::Leaf)
                .collect(),
        });
    }

    /// Flatten all capabilities into the transport output format.
    pub fn flatten(&self) -> Vec<CapabilityRef> {
        let mut refs = Vec::new();
        for node in &self.root {
            node.flatten_into(&mut refs);
        }
        refs.into_iter().cloned().collect()
    }

    /// Returns the internal root nodes for structured callers.
    pub fn roots(&self) -> &[AppCapabilityNode] {
        &self.root
    }
}
