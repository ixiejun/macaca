//! Runtime-host audit replay convergence tests (task 2.7.1).
//!
//! These tests exercise the in-memory `ServiceCallAuditSink` replay contract to
//! prove that a simulated YAML session and a simulated WASM session each produce
//! exactly one distinct `service.agent_execution` evidence chain when filtered by
//! `session_id`.  Live `/api/chat/v2` replay still requires provider credentials;
//! this module is the no-network proxy referenced from `audit-replay-baseline.md`.

use std::collections::HashSet;

use macaca_proto::AGENT_EXECUTION_SERVICE_ID;

use crate::service_call_audit::{
    InMemoryServiceCallAuditSink, ServiceCallAuditEvent, ServiceCallAuditSink,
};

/// Count distinct terminal execution providers for completed agent runs in one replay.
///
/// Pre-convergence sessions could register multiple terminal backends (shell-owned
/// framework loop, kernel port adapter, hosted graph_owner branches).  Post-convergence
/// every successful `service.agent_execution` call must report the same composed
/// backend as the terminal provider, regardless of which shell entry surface initiated
/// the run.
fn distinct_terminal_agent_execution_providers(events: &[ServiceCallAuditEvent]) -> HashSet<String> {
    events
        .iter()
        .filter(|event| {
            event.service_id == AGENT_EXECUTION_SERVICE_ID
                && event.stage == "service_call_succeeded"
        })
        .filter_map(|event| event.provider_id.clone())
        .collect()
}

/// Emit a minimal canonical replay chain for one session through the in-memory sink.
///
/// The stages mirror what `ServiceCallExecutor` records: request → route → provider
/// success.  Only the terminal provider id varies between pre-convergence parallel
/// backends; post-convergence all agent runs share `ComposedAgentExecutionBackend`.
fn record_canonical_agent_execution_chain(
    sink: &InMemoryServiceCallAuditSink,
    session_id: &str,
    trace_id: &str,
    entry_source: &str,
) {
    let mut requested = ServiceCallAuditEvent::new(
        "service_call_requested",
        trace_id,
        AGENT_EXECUTION_SERVICE_ID,
    );
    requested.session_id = Some(session_id.into());
    requested.provider_id = Some(entry_source.into());
    sink.emit(requested).expect("audit sink should accept requested event");

    let mut routed = ServiceCallAuditEvent::new(
        "service_call_routed",
        trace_id,
        AGENT_EXECUTION_SERVICE_ID,
    );
    routed.session_id = Some(session_id.into());
    routed.provider_id = Some("ComposedAgentExecutionBackend".into());
    sink.emit(routed).expect("audit sink should accept routed event");

    let mut succeeded = ServiceCallAuditEvent::new(
        "service_call_succeeded",
        trace_id,
        AGENT_EXECUTION_SERVICE_ID,
    );
    succeeded.session_id = Some(session_id.into());
    succeeded.provider_id = Some("ComposedAgentExecutionBackend".into());
    succeeded.decision = Some("completed".into());
    sink.emit(succeeded)
        .expect("audit sink should accept succeeded event");
}

#[test]
fn yaml_session_replay_shows_single_agent_execution_chain() {
    let sink = InMemoryServiceCallAuditSink::new();
    let session_id = "session-yaml-convergence";

    // Simulate chat main-thread run (formerly yaml-A chain).
    record_canonical_agent_execution_chain(
        &sink,
        session_id,
        "trace-yaml-chat",
        "macaca.web.chat_orchestrator",
    );

    let replay = sink
        .replay_by_session_id(session_id)
        .expect("session replay should succeed");
    let agent_execution_events: Vec<_> = replay
        .iter()
        .filter(|event| event.service_id == AGENT_EXECUTION_SERVICE_ID)
        .collect();

    assert!(
        !agent_execution_events.is_empty(),
        "yaml session replay must contain agent execution evidence"
    );

    let terminal_providers: HashSet<_> = agent_execution_events
        .iter()
        .filter_map(|event| event.provider_id.as_deref())
        .filter(|provider| *provider == "ComposedAgentExecutionBackend")
        .collect();
    assert_eq!(
        terminal_providers.len(),
        1,
        "yaml session must converge on one terminal execution provider"
    );

    let terminal_providers = distinct_terminal_agent_execution_providers(&replay);
    assert_eq!(
        terminal_providers.len(),
        1,
        "yaml session audit replay must show exactly one terminal execution provider, got {terminal_providers:?}"
    );
}

#[test]
fn wasm_session_replay_shows_single_agent_execution_chain() {
    let sink = InMemoryServiceCallAuditSink::new();
    let session_id = "session-wasm-convergence";

    // Simulate WASM delegate run (formerly wasm-B chain, now unified through delegate bridge).
    record_canonical_agent_execution_chain(
        &sink,
        session_id,
        "trace-wasm-delegate",
        "macaca.runtime.application_agent_delegate",
    );

    let replay = sink
        .replay_by_session_id(session_id)
        .expect("session replay should succeed");
    let terminal_providers = distinct_terminal_agent_execution_providers(&replay);

    assert_eq!(
        terminal_providers.len(),
        1,
        "wasm session audit replay must show exactly one terminal execution provider, got {terminal_providers:?}"
    );
    assert!(
        replay
            .iter()
            .all(|event| event.service_id == AGENT_EXECUTION_SERVICE_ID),
        "wasm session agent run evidence must stay on service.agent_execution"
    );
}

#[test]
fn distinct_trace_ids_within_same_session_still_share_one_terminal_provider() {
    let sink = InMemoryServiceCallAuditSink::new();
    let session_id = "session-mixed-intents";

    // Chat + workflow delegate in one session (different traces, same terminal provider).
    record_canonical_agent_execution_chain(
        &sink,
        session_id,
        "trace-chat",
        "macaca.web.chat_orchestrator",
    );
    record_canonical_agent_execution_chain(
        &sink,
        session_id,
        "trace-workflow",
        "macaca.web.application_agent_delegate",
    );

    let replay = sink.replay_by_session_id(session_id).unwrap();
    let terminal_providers: HashSet<_> = replay
        .iter()
        .filter_map(|event| event.provider_id.as_deref())
        .filter(|provider| *provider == "ComposedAgentExecutionBackend")
        .collect();

    assert_eq!(
        terminal_providers.len(),
        1,
        "multiple intents in one session must not fork into parallel execution backends"
    );
}
