//! Agent capability composite primitives.

use macaca_proto::Capability;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilitySource {
    Direct,
    Manifest,
    Persona,
    Skill,
    Driver,
    Mcp,
}

#[derive(Debug, Clone)]
pub enum AgentCapabilityNode {
    Leaf(Capability),
    Group {
        source: CapabilitySource,
        children: Vec<AgentCapabilityNode>,
    },
}

impl AgentCapabilityNode {
    fn flatten_into(&self, out: &mut Vec<Capability>) {
        match self {
            Self::Leaf(capability) => out.push(capability.clone()),
            Self::Group { children, .. } => {
                for child in children {
                    child.flatten_into(out);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentCapabilitySet {
    nodes: Vec<AgentCapabilityNode>,
}

impl AgentCapabilitySet {
    /// Build a capability set from a flat provider-neutral capability list.
    pub fn from_flat_capabilities(capabilities: Vec<Capability>) -> Self {
        Self::from_source(CapabilitySource::Direct, capabilities)
    }

    pub fn from_source(source: CapabilitySource, capabilities: Vec<Capability>) -> Self {
        let mut set = Self::default();
        set.push_group(source, capabilities);
        set
    }

    pub fn push_group(&mut self, source: CapabilitySource, children: Vec<Capability>) {
        self.nodes.push(AgentCapabilityNode::Group {
            source,
            children: children
                .into_iter()
                .map(AgentCapabilityNode::Leaf)
                .collect(),
        });
    }

    /// Flatten grouped capability declarations into the canonical public list.
    pub fn flatten(&self) -> Vec<Capability> {
        let mut flattened = Vec::new();
        for node in &self.nodes {
            node.flatten_into(&mut flattened);
        }
        flattened
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.flatten().len()
    }

    pub fn nodes(&self) -> &[AgentCapabilityNode] {
        &self.nodes
    }

    pub fn sources(&self) -> Vec<CapabilitySource> {
        self.nodes
            .iter()
            .map(|node| match node {
                AgentCapabilityNode::Leaf(_) => CapabilitySource::Direct,
                AgentCapabilityNode::Group { source, .. } => *source,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability(name: &str) -> Capability {
        Capability {
            name: name.into(),
            description: format!("{name} capability"),
        }
    }

    #[test]
    fn capability_set_reports_sources_without_changing_flattened_output() {
        let mut set = AgentCapabilitySet::from_flat_capabilities(vec![capability("direct")]);
        set.push_group(CapabilitySource::Skill, vec![capability("browser")]);
        set.push_group(CapabilitySource::Driver, vec![capability("opencode")]);

        let flattened = set.flatten();

        assert_eq!(set.len(), 3);
        assert_eq!(
            set.sources(),
            vec![
                CapabilitySource::Direct,
                CapabilitySource::Skill,
                CapabilitySource::Driver,
            ]
        );
        assert_eq!(
            flattened
                .iter()
                .map(|capability| capability.name.as_str())
                .collect::<Vec<_>>(),
            vec!["direct", "browser", "opencode"]
        );
    }

    #[test]
    fn capability_set_from_source_matches_push_group() {
        let from_source =
            AgentCapabilitySet::from_source(CapabilitySource::Skill, vec![capability("browser")]);

        let mut pushed = AgentCapabilitySet::default();
        pushed.push_group(CapabilitySource::Skill, vec![capability("browser")]);

        assert_eq!(
            from_source
                .flatten()
                .iter()
                .map(|capability| capability.name.as_str())
                .collect::<Vec<_>>(),
            pushed
                .flatten()
                .iter()
                .map(|capability| capability.name.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(from_source.sources(), pushed.sources());
    }
}
