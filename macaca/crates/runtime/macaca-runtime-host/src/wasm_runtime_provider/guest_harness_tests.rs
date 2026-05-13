use macaca_proto::{
    ApplicationHostCommandStatus, ApplicationImport, WASM_HOST_IMPORT_CAPABILITY,
    WASM_HOST_IMPORT_OPERATION, WASM_HOST_IMPORT_SERVICE_ID,
};
use serde_json::json;

use super::{WasmExampleFixtureKind, WasmGuestRuntimeHarness, WasmMockHostOutcome};

#[test]
fn wasm_guest_sdk_service_proxy_builds_traced_provider_neutral_command() {
    let harness = WasmGuestRuntimeHarness::new("fixture.wasm_guest_sdk");

    let command = harness.service_call(
        "fixture.service",
        "query",
        "fixture.capability",
        json!({
            "input": true,
            "raw_payload": "must not survive",
            "secret": "must not survive"
        }),
    );

    assert_eq!(command.import, ApplicationImport::ServiceCall);
    assert!(command
        .trace
        .unwrap()
        .trace_id
        .contains("fixture.wasm_guest_sdk"));
    assert_eq!(
        command
            .metadata
            .get(WASM_HOST_IMPORT_SERVICE_ID)
            .map(String::as_str),
        Some("fixture.service")
    );
    assert_eq!(
        command
            .metadata
            .get(WASM_HOST_IMPORT_OPERATION)
            .map(String::as_str),
        Some("query")
    );
    assert_eq!(
        command
            .metadata
            .get(WASM_HOST_IMPORT_CAPABILITY)
            .map(String::as_str),
        Some("fixture.capability")
    );
    assert_eq!(command.payload["input"], json!(true));
    assert!(command.payload.get("raw_payload").is_none());
    assert!(command.payload.get("secret").is_none());
}

#[test]
fn wasm_local_harness_mock_host_imports_cover_success_denied_unavailable_and_unsupported() {
    let harness = WasmGuestRuntimeHarness::new("fixture.wasm_local_harness")
        .mock_outcome(
            ApplicationImport::ServiceCall,
            "fixture.service",
            "ok",
            WasmMockHostOutcome::Success(json!({"accepted": true, "prompt": "redact"})),
        )
        .mock_outcome(
            ApplicationImport::ServiceCall,
            "fixture.service",
            "denied",
            WasmMockHostOutcome::Denied("policy denied secret".into()),
        )
        .mock_outcome(
            ApplicationImport::ServiceCall,
            "fixture.service",
            "unavailable",
            WasmMockHostOutcome::Unavailable("service unavailable".into()),
        );

    let ok = harness.dispatch(harness.service_call(
        "fixture.service",
        "ok",
        "fixture.capability",
        json!({}),
    ));
    assert!(matches!(ok.result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        ok.result.metadata.get("reason_code").map(String::as_str),
        Some("mock_import_completed")
    );
    assert!(ok.result.output.get("prompt").is_none());

    let denied = harness.dispatch(harness.service_call(
        "fixture.service",
        "denied",
        "fixture.capability",
        json!({}),
    ));
    assert!(matches!(
        denied.result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        denied
            .result
            .metadata
            .get("reason_code")
            .map(String::as_str),
        Some("mock_import_denied")
    );

    let unavailable = harness.dispatch(harness.service_call(
        "fixture.service",
        "unavailable",
        "fixture.capability",
        json!({}),
    ));
    assert!(matches!(
        unavailable.result.status,
        ApplicationHostCommandStatus::Unavailable { .. }
    ));

    let unsupported = harness.dispatch(harness.storage_get("fixture/key"));
    assert!(matches!(
        unsupported.result.status,
        ApplicationHostCommandStatus::Unsupported { .. }
    ));
    assert_eq!(
        unsupported
            .result
            .metadata
            .get("reason_code")
            .map(String::as_str),
        Some("mock_import_unsupported")
    );
}

#[test]
fn wasm_local_harness_rejects_missing_trace_without_leaking_payload() {
    let harness = WasmGuestRuntimeHarness::new("fixture.wasm_missing_trace");
    let mut command = harness.service_call(
        "fixture.service",
        "ok",
        "fixture.capability",
        json!({"raw_payload": "secret"}),
    );
    command.trace = None;

    let report = harness.dispatch(command);

    assert!(matches!(
        report.result.status,
        ApplicationHostCommandStatus::Rejected { .. }
    ));
    assert_eq!(
        report
            .result
            .metadata
            .get("reason_code")
            .map(String::as_str),
        Some("missing_trace")
    );
    assert!(report.result.output.is_null());
}

#[test]
fn wasm_toolchain_wit_labels_match_application_abi_labels() {
    let harness = WasmGuestRuntimeHarness::new("fixture.wasm_toolchain");
    let report = harness.wit_label_report();

    assert!(report.is_consistent(), "{report:?}");
    assert!(report
        .wit_imports
        .contains(&ApplicationImport::ServiceCall.as_name().to_string()));
    assert!(report.wit_exports.contains(&"app:start".to_string()));
}

#[test]
fn wasm_toolchain_generates_deterministic_example_fixtures() {
    let harness = WasmGuestRuntimeHarness::new("fixture.wasm_toolchain_examples");
    let kinds = [
        WasmExampleFixtureKind::Headless,
        WasmExampleFixtureKind::GenUiRender,
        WasmExampleFixtureKind::MemoryContextImport,
        WasmExampleFixtureKind::ServiceUnavailable,
    ];

    for kind in kinds {
        let first = harness.example_fixture(kind.clone());
        let second = harness.example_fixture(kind);

        assert_eq!(first.fixture_id, second.fixture_id);
        assert_eq!(first.manifest, second.manifest);
        assert_eq!(first.artifact, second.artifact);
        assert_eq!(first.commands, second.commands);
        assert_eq!(first.manifest.runtime.kind.to_string(), "wasm_component");
        assert!(!first.artifact.digest.value.is_empty());
        assert!(!first.artifact.required_imports.is_empty());
        assert!(!first.commands.is_empty());
        assert!(!format!("{first:?}").contains("raw_payload"));
    }
}

#[test]
fn wasm_guest_sdk_render_and_trace_facades_emit_traced_commands() {
    let harness = WasmGuestRuntimeHarness::new("fixture.wasm_guest_sdk_facades");

    let render = harness.render("main", json!({"state": "ready"}));
    let trace = harness.trace_emit(json!({"event": "ready"}));

    assert_eq!(render.import, ApplicationImport::UiRender);
    assert_eq!(trace.import, ApplicationImport::TraceEmit);
    assert!(render.trace.is_some());
    assert!(trace.trace.is_some());
}
