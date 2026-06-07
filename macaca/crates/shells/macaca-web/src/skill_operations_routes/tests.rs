//! Contract and adapter-behavior tests for skill-operations routes.

use std::path::PathBuf;

use super::adapter::materialization_collection_root_from_workspace_path;
use super::contract_source::skill_operations_module_sources;

#[test]
fn skill_operations_routes_remain_thin_sdk_adapters() {
    let source = skill_operations_module_sources();
    assert!(source.contains(".skill_client"));
    assert!(source.contains("curation_lifecycle"));
    assert!(source.contains("promote_skill_draft"));
    assert!(source.contains("reject_skill_draft"));
    assert!(source.contains("self_evolution_evaluation_report"));
    assert!(source.contains("evaluate_self_evolution"));
    assert!(!source.contains(&format!("{}{}", "SelfEvolutionScoringPolicy", "::score")));
    assert!(!source.contains(&format!("{}{}", "macaca_runtime", "_host::")));
    assert!(!source.contains(&format!("{}{}", "SkillGovernance", "StoreStrategy")));
    assert!(!source.contains(&format!("{}{}", "SkillLifecycleState::", "Archived")));
}

#[test]
fn default_materialization_root_uses_workspace_skills_source() {
    let workspace_root = PathBuf::from("/tmp/macaca-workspace/app-id");

    let root = materialization_collection_root_from_workspace_path(&workspace_root);

    assert_eq!(root, workspace_root.join("skills"));
    assert!(
        !root.ends_with("available_skills"),
        "available_skills is a projection output and must not be the default package source"
    );
}
