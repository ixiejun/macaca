//! Manifest admission Specification for knowledge-graph permission scopes.
//!
//! The validator owns only descriptor vocabulary checks. Query policy, resource
//! budgets, approvals, and provider Strategy selection remain runtime-host
//! responsibilities.

use super::knowledge_graph::{GRAPH_PERMISSION_SCOPES, KNOWLEDGE_GRAPH_PACK_ID};
use super::model::AppServiceContractConfig;

/// Reject graph permission declarations outside the pack descriptor vocabulary.
pub fn validate_knowledge_graph_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get(KNOWLEDGE_GRAPH_PACK_ID)
    else {
        return Ok(());
    };
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack| pack == KNOWLEDGE_GRAPH_PACK_ID);
    if !declared {
        return Err("knowledge graph permissions require the knowledge graph pack");
    }
    if scopes
        .iter()
        .any(|scope| !GRAPH_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("knowledge graph permission scope is not declared by the pack");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn graph_permissions_are_descriptor_owned() {
        let declaration = AppServiceContractConfig {
            optional_packs: vec![KNOWLEDGE_GRAPH_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                KNOWLEDGE_GRAPH_PACK_ID.into(),
                BTreeSet::from(["graph.query".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_knowledge_graph_permission_declarations(&declaration).is_ok());
        let unknown = AppServiceContractConfig {
            optional_packs: vec![KNOWLEDGE_GRAPH_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                KNOWLEDGE_GRAPH_PACK_ID.into(),
                BTreeSet::from(["graph.native_database".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_knowledge_graph_permission_declarations(&unknown).is_err());
    }
}
