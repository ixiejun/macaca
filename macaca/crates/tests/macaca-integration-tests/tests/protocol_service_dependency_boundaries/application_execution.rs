use std::collections::BTreeSet;

use super::gate;

/// Verify that the generic protocol microkernel dependency gate contains application
/// execution-specific ownership rules.
///
/// This is an executable Specification check rather than a prose-only reminder:
/// the proposal requires kernel, SDK/SystemFacade, and shell boundaries to stay
/// out of concrete application execution provider ownership. The rule ids are
/// intentionally stable because they also appear in diagnostics when a future
/// dependency edge violates the architecture contract.
#[test]
fn application_execution_boundary_rules_are_registered() {
    let rule_ids = gate::boundary_rule_ids()
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert!(
        rule_ids.contains("application-execution-kernel-no-provider-deps"),
        "kernel crates must have an explicit guard against depending on application execution provider implementations"
    );
    assert!(
        rule_ids.contains("application-execution-sdk-no-runtime-provider-construction"),
        "SDK/SystemFacade crates must have an explicit guard against constructing runtime-host application execution providers"
    );
    assert!(
        rule_ids.contains("application-execution-shell-no-semantic-owner"),
        "Web/CLI shells must have an explicit guard against owning application execution semantics"
    );
}
