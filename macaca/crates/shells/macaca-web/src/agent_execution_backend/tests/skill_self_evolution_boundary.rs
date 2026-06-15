//! Agent execution backend test module — skill self-evolution boundary contract.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.

use super::support::*;
#[test]
fn skill_self_evolution_observation_is_centralized_at_agent_execution_boundary() {
    let composed_source = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/composed_agent_execution_backend.rs"
    );
    let chat_source = crate::chat_orchestrator::contract_source::chat_orchestrator_module_sources();
    let event_persistence_source = include_str!("../../event_persistence.rs");
    let observer_source =
        crate::skill_self_evolution_observer::contract_source::skill_self_evolution_observer_module_sources();

    assert!(
        !composed_source.contains("spawn_skill_self_evolution_observation"),
        "composed agent execution backend should return results; the decorator owns observation"
    );
    assert!(
        !chat_source.contains("observe_agent_execution_result_for_skill_self_evolution"),
        "chat orchestration must not duplicate Skill self-evolution observation"
    );
    assert!(
        !event_persistence_source.contains("observe_executor_event_for_skill_self_evolution"),
        "event persistence must remain durable logging, not Skill proposal ownership"
    );
    assert!(
        !observer_source.contains("observe_executor_event_for_skill_self_evolution"),
        "observer helpers must expose the service.agent_execution result boundary only"
    );
}
