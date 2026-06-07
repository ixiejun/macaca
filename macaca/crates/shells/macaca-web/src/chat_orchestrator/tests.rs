use serde_json::json;

use super::contract_source::chat_orchestrator_module_sources;
use super::wasm_dispatch_adapter::{
    new_session_preparation_for_chat, wasm_chat_export_payload, NewSessionPreparation,
};

#[test]
fn wasm_with_declared_agents_prepares_orchestration_executor() {
    assert_eq!(
        new_session_preparation_for_chat(true, true),
        NewSessionPreparation::WasmOrchestrationExecutor
    );
}

#[test]
fn agentless_wasm_keeps_host_dispatch_only() {
    assert_eq!(
        new_session_preparation_for_chat(true, false),
        NewSessionPreparation::WasmHostDispatchOnly
    );
}

#[test]
fn agentless_wasm_updates_entry_agent_activity_without_app_specific_branch() {
    let source = chat_orchestrator_module_sources();

    assert!(source.contains("update_agent_activity_by_name"));
    assert!(source.contains("sync_delegated_agent_activity_from_executor_event"));
    assert!(source.contains("Handling WASM session"));
    assert!(source.contains("macaca_proto::AgentActivity::Working"));
    assert!(source.contains("macaca_proto::AgentActivity::Idle"));
    assert!(source.contains("macaca_proto::AgentActivity::Error"));
    let app_specific_name = ["wasm-crypto", "-signal-app"].concat();
    assert!(!source.contains(&app_specific_name));
}

#[test]
fn non_wasm_chat_uses_framework_executor() {
    assert_eq!(
        new_session_preparation_for_chat(false, true),
        NewSessionPreparation::FrameworkExecutor
    );
    assert_eq!(
        new_session_preparation_for_chat(false, false),
        NewSessionPreparation::FrameworkExecutor
    );
}

#[test]
fn wasm_chat_payload_preserves_app_owned_typed_fields() {
    let payload = wasm_chat_export_payload(r#"{"input":"Analyze BTC/USDT","symbol":"BTC"}"#);

    assert_eq!(payload["input"], json!("Analyze BTC/USDT"));
    assert_eq!(payload["symbol"], json!("BTC"));
    assert_eq!(payload["channel"], json!("chat"));
}

#[test]
fn wasm_chat_payload_keeps_plain_prompt_untyped() {
    let payload = wasm_chat_export_payload("Analyze BTC/USDT");

    assert_eq!(payload["input"], json!("Analyze BTC/USDT"));
    assert!(payload.get("symbol").is_none());
    assert_eq!(payload["channel"], json!("chat"));
}

#[test]
fn chat_main_thread_enters_agent_execution_service_boundary() {
    let source = chat_orchestrator_module_sources();
    let legacy_builder = ["FrameworkRunner::build_", "coordinator"].concat();

    assert!(source.contains("run_chat_main_thread_via_agent_service"));
    assert!(source.contains("AgentExecutionIntent::ChatMainThread"));
    assert!(source.contains("AGENT_EXECUTION_SERVICE_ID"));
    assert!(!source.contains(&legacy_builder));
}

#[test]
fn chat_done_event_is_after_session_memory_capture_attempt() {
    let source = chat_orchestrator_module_sources();
    let capture = "capture_successful_session_completion";
    let done = r#".event("done")"#;
    let capture_index = source
        .find(capture)
        .expect("chat completion should attempt sanitized memory capture");
    let done_index = source[capture_index..]
        .find(done)
        .expect("chat completion should emit done after memory capture")
        + capture_index;

    assert!(
        capture_index < done_index,
        "the public done event must remain a durable boundary after memory capture"
    );
}
