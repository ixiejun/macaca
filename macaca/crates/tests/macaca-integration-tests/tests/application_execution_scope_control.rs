//! Application execution protocol platform scope-control gate.
//!
//! This integration test keeps the generic application execution platform from
//! regressing into application-specific routing. The OS and service/runtime
//! layers may carry application ids as data, but they must not branch on product
//! names, workflow names, provider names, driver names, gateway names, model
//! names, or business domains.

#[path = "application_execution_scope_control/gate.rs"]
mod gate;

#[test]
fn application_execution_scope_control_rejects_product_identity_branches() {
    gate::assert_application_execution_scope_control();
}
