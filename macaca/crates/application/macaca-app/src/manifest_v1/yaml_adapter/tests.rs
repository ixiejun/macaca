//! Contract tests for YAML → Manifest v1 adapter projection boundaries.
//!
//! Fixtures use neutral `fixture-*` identifiers so migration-debt inventory
//! scans do not flag forbidden production role tokens.

use macaca_proto::{AbilityImplementationKind, ApplicationAbilityKind};

use crate::model::{AgentSource, AppLayer, CapabilityRef, InlineAgentConfig};

use super::types::YamlApplicationManifestAdapter;

#[test]
fn yaml_projection_creates_agent_ability_for_inline_agent() {
    let manifest = crate::model::AppManifest {
        id: macaca_proto::ApplicationId::new(),
        name: "fixture".into(),
        description: None,
        version: "1.0.0".into(),
        layer: AppLayer::L3Declarative,
        ui_type: None,
        agents: vec![AgentSource::Inline(InlineAgentConfig {
            name: "fixture-role-alpha".into(),
            capabilities: vec![CapabilityRef {
                name: "capability.fixture".into(),
                description: "Fixture capability".into(),
            }],
            prompt_template: "never serialized into projection metadata".into(),
            model: "model.fixture".into(),
            permission_level: "user".into(),
            allowed_tools: vec!["tool.fixture".into()],
            max_tokens: None,
            temperature: None,
            skills: None,
            context_engine: None,
        })],
        llm_config: None,
        entry_agent: Some("fixture-role-alpha".into()),
        entrypoint: None,
        workflows: None,
        resources: None,
        context: None,
        service_contract: None,
        execution_profile: None,
        workbench: None,
        autonomy: None,
        ui: None,
        execution_control: None,
    };

    let projection = YamlApplicationManifestAdapter::new(manifest).project();

    assert_eq!(projection.manifest.abilities.len(), 1);
    assert_eq!(
        projection.manifest.abilities[0].kind,
        ApplicationAbilityKind::Agent
    );
    assert!(projection.manifest.abilities[0]
        .capabilities
        .iter()
        .any(|capability| capability.id.as_str().contains("capability.fixture")));
    assert!(!projection
        .manifest
        .metadata
        .values()
        .any(|value| value.contains("never serialized")));
}

#[test]
fn yaml_projection_synthesizes_wasm_runtime_ability_from_service_contract() {
    let manifest = crate::model::AppManifest {
        id: macaca_proto::ApplicationId::new(),
        name: "wasm-fixture".into(),
        description: None,
        version: "1.0.0".into(),
        layer: AppLayer::L2Wasm,
        ui_type: None,
        agents: Vec::new(),
        llm_config: None,
        entry_agent: None,
        entrypoint: None,
        workflows: None,
        resources: None,
        context: None,
        service_contract: Some(crate::service_capability::AppServiceContractConfig {
            use_packs: vec!["pack.finance.v1".into()],
            required_services: vec!["service.custom.required".into()],
            optional_services: Vec::new(),
            service_policy_overrides: Default::default(),
        }),
        execution_profile: None,
        workbench: None,
        autonomy: None,
        ui: None,
        execution_control: None,
    };
    let projection = YamlApplicationManifestAdapter::new(manifest).project();
    let ability = projection
        .manifest
        .abilities
        .iter()
        .find(|ability| ability.id == "ability.runtime.wasm")
        .expect("expected synthesized wasm runtime ability");
    assert_eq!(ability.kind, ApplicationAbilityKind::Headless);
    assert_eq!(
        ability.implementation,
        AbilityImplementationKind::WasmComponent
    );
    assert!(ability
        .services
        .iter()
        .any(|service| service.service.as_str() == "service.custom.required"));
    assert!(
        !ability.services.iter().any(|service| {
            service.service.as_str() == "service.market_data"
        }),
        "finance pack services must not expand without an installed catalog entry"
    );
}
