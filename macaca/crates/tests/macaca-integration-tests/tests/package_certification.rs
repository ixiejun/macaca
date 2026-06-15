//! Package certification tests for the protocol-driven application path.
//!
//! These tests are intentionally no-network and provider-neutral.  They use a
//! Template Method style harness: each package class supplies a fixture and an
//! assertion closure, while the harness executes the same conformance checker
//! flow and verifies that certification always returns structured diagnostics
//! instead of panicking, hanging, or silently passing invalid metadata.

use macaca_app::{ConformanceHostContext, ConformanceStatus, PackageConformanceChecker};
use macaca_proto::EntitlementState;
use macaca_sdk::{
    driver_plugin_fixture, evm_optional_fixture, gateway_plugin_fixture, genui_app_fixture,
    invalid_missing_required_service_fixture, invalid_missing_runtime_fixture, paid_skill_fixture,
    wasm_stub_app_fixture, web3_optional_fixture, yaml_app_fixture,
};

fn host_context() -> ConformanceHostContext {
    ConformanceHostContext::default()
        .with_service("service.agent.runtime")
        .with_service("service.application.abi")
        .with_service("service.ui.runtime")
        .with_service("service.gateway.registry")
        .with_service("service.driver.registry")
}

fn certify(
    name: &str,
    descriptor: macaca_proto::PackageDescriptor,
    context: ConformanceHostContext,
) -> macaca_app::ConformanceReport {
    let report = PackageConformanceChecker::new().check(&descriptor, &context);
    assert!(
        !report.trace_events.is_empty(),
        "{name} certification must produce trace events"
    );
    assert!(
        report
            .trace_events
            .iter()
            .any(|event| event.rule == "checker"),
        "{name} certification must include checker lifecycle trace"
    );
    report
}

#[test]
fn package_certification_accepts_yaml_application_fixture() {
    let report = certify("yaml app", yaml_app_fixture(), host_context());
    assert_eq!(report.status, ConformanceStatus::Conformant);
}

#[test]
fn package_certification_accepts_wasm_stub_as_metadata_only() {
    let report = certify("wasm stub", wasm_stub_app_fixture(), host_context());
    assert_eq!(report.status, ConformanceStatus::Conformant);
    assert_eq!(report.runtime_kind.unwrap().to_string(), "wasm_component");
}

#[test]
fn package_certification_accepts_traced_genui_fixture() {
    let report = certify("genui app", genui_app_fixture(), host_context());
    assert_eq!(report.status, ConformanceStatus::Conformant);
    assert!(report
        .trace_events
        .iter()
        .any(|event| event.rule == "trace" && event.outcome == "passed"));
}

#[test]
fn package_certification_accepts_gateway_and_driver_plugins() {
    let gateway = certify("gateway plugin", gateway_plugin_fixture(), host_context());
    let driver = certify("driver plugin", driver_plugin_fixture(), host_context());

    assert_eq!(gateway.status, ConformanceStatus::Conformant);
    assert_eq!(driver.status, ConformanceStatus::Conformant);
}

#[test]
fn package_certification_reports_paid_skill_entitlement_states() {
    let denied = certify("paid skill denied", paid_skill_fixture(), host_context());
    assert_eq!(denied.status, ConformanceStatus::ConformantWithWarnings);
    assert!(denied
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "commerce.entitlement_missing"));

    let allowed_context = host_context().with_entitlement_state(EntitlementState::valid());
    let allowed = certify("paid skill allowed", paid_skill_fixture(), allowed_context);
    assert_eq!(allowed.status, ConformanceStatus::Conformant);
}

#[test]
fn package_certification_keeps_web3_and_evm_optional_modules_unavailable_safe() {
    let web3 = certify("optional web3", web3_optional_fixture(), host_context());
    let evm = certify("optional evm", evm_optional_fixture(), host_context());

    assert_eq!(web3.status, ConformanceStatus::ConformantWithWarnings);
    assert_eq!(evm.status, ConformanceStatus::ConformantWithWarnings);
    assert!(web3
        .diagnostics
        .iter()
        .chain(evm.diagnostics.iter())
        .any(|diagnostic| diagnostic.code == "optional_module.unavailable"));
}

#[test]
fn package_certification_rejects_invalid_fixtures_with_structured_diagnostics() {
    let missing_runtime = certify(
        "missing runtime",
        invalid_missing_runtime_fixture(),
        host_context(),
    );
    let missing_service = certify(
        "missing service",
        invalid_missing_required_service_fixture(),
        host_context(),
    );

    assert_eq!(missing_runtime.status, ConformanceStatus::NonConformant);
    assert_eq!(missing_service.status, ConformanceStatus::NonConformant);
    assert!(missing_runtime
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime.missing_kind"));
    assert!(missing_service
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "service.required_missing"));
}
