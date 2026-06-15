//! Policy and adapter tests for application-owned UI routes.

use std::collections::BTreeSet;
use std::path::Path;

use axum::http::StatusCode;
use macaca_proto::{ApplicationHostCommandStatus, TraceContext};

use super::adapter::{ensure_declared_asset, normalize_package_path};
use super::bridge_adapter::{rejected_bridge_result, rejected_undeclared_service_result};
use super::types::AppUiRouteRuntime;

fn ui_config() -> AppUiRouteRuntime {
    AppUiRouteRuntime {
        entry: Some("dist/ui/index.html".into()),
        assets: vec!["dist/ui/assets/**".into()],
        bridge_required: vec!["service.call".into()],
        bridge_optional: vec!["trace.emit".into()],
    }
}

#[test]
fn app_ui_asset_policy_allows_entry_and_declared_asset_prefix() {
    let ui = ui_config();
    assert!(ensure_declared_asset(&ui, Path::new("dist/ui/index.html")).is_ok());
    assert!(ensure_declared_asset(&ui, Path::new("dist/ui/assets/app.js")).is_ok());
}

#[test]
fn app_ui_asset_policy_rejects_undeclared_package_file() {
    let error = ensure_declared_asset(&ui_config(), Path::new("dist/secret.txt"))
        .expect_err("undeclared package files must be rejected");
    assert_eq!(error.0, StatusCode::FORBIDDEN);
}

#[test]
fn app_ui_asset_path_normalization_rejects_escape() {
    let error = normalize_package_path("../secret.txt")
        .expect_err("path traversal must be rejected before filesystem access");
    assert_eq!(error.0, StatusCode::FORBIDDEN);
}

#[test]
fn app_ui_bridge_service_policy_rejects_undeclared_service() {
    let declared = BTreeSet::from(["service.file".to_string()]);
    let result = rejected_undeclared_service_result(
        "service.llm",
        &declared,
        TraceContext::new("test-ui-bridge-service-policy"),
    )
    .expect("undeclared service calls must fail before host dispatch");
    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some("service.llm")
    );
}

#[test]
fn app_ui_bridge_service_policy_allows_declared_service() {
    let declared = BTreeSet::from(["service.llm".to_string()]);
    assert!(rejected_undeclared_service_result(
        "service.llm",
        &declared,
        TraceContext::new("test-ui-bridge-service-policy-ok"),
    )
    .is_none());
}

#[test]
fn app_ui_bridge_rejection_is_structured_and_traced() {
    let result = rejected_bridge_result("capability denied", TraceContext::new("test-ui-bridge"));
    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("ui_bridge_rejected")
    );
}
