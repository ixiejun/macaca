use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::finance_accounting::FINANCE_ACCOUNTING_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::finance_accounting_service_provider::{
    FinanceAccountingRuntimeEventKind, FinanceAccountingSystemServiceProvider,
};

#[tokio::test]
async fn accounting_provider_dispatches_contract_commands_without_payload_echo() {
    let provider = FinanceAccountingSystemServiceProvider::mock();
    for command in FINANCE_ACCOUNTING_COMMANDS {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({
                    "account_number": "secret-marker",
                    "raw_ledger": "payload-marker",
                }),
                TraceContext::new(format!("trace-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.status, "ok");
        assert!(!result.output.to_string().contains("marker"));
    }
    assert_eq!(
        provider.capability().supported_commands.len(),
        FINANCE_ACCOUNTING_COMMANDS.len()
    );
}

#[tokio::test]
async fn accounting_provider_strategy_replacement_exposes_capability_gap() {
    let provider = FinanceAccountingSystemServiceProvider::mock_with_commands([
        "accounting.get_chart_of_accounts",
    ]);
    assert!(provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("accounting.get_chart_of_accounts"),
            serde_json::json!({}),
            TraceContext::new("accounting-read"),
        ))
        .await
        .is_ok());
    assert!(matches!(
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("accounting.post_journal"),
                serde_json::json!({}),
                TraceContext::new("accounting-gap"),
            ))
            .await,
        Err(ServiceError::UnsupportedCommand(code)) if code == "accounting_command_unsupported"
    ));
}

#[tokio::test]
async fn accounting_provider_admission_denies_before_retaining_reference() {
    let provider = FinanceAccountingSystemServiceProvider::mock();
    for payload in [
        serde_json::json!({"policy_denied": true}),
        serde_json::json!({"approval_required": true}),
        serde_json::json!({"period_locked": true}),
        serde_json::json!({"stale_data": true}),
    ] {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new("accounting.post_journal"),
                    payload,
                    TraceContext::new("accounting-denied"),
                ))
                .await,
            Err(ServiceError::DisabledByPolicy(_))
        ));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn accounting_provider_unavailable_is_explicit_for_every_command() {
    let provider = FinanceAccountingSystemServiceProvider::unavailable("module_absent");
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    for command in FINANCE_ACCOUNTING_COMMANDS {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({}),
                    TraceContext::new(format!("unavailable-{command}")),
                ))
                .await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
}

#[tokio::test]
async fn accounting_provider_snapshot_and_shutdown_are_bounded() {
    let provider = FinanceAccountingSystemServiceProvider::mock();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("accounting.inspect_provider"),
            serde_json::json!({}),
            TraceContext::new("snapshot-accounting"),
        ))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
    provider.shutdown().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn accounting_provider_emits_replayable_side_effect_events() {
    let provider = FinanceAccountingSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("accounting.post_journal"),
            serde_json::json!({}),
            TraceContext::new("replay-accounting"),
        ))
        .await
        .unwrap();
    let mut saw_planned = false;
    let mut saw_approved = false;
    while let Ok(event) = events.try_recv() {
        saw_planned |= event.kind == FinanceAccountingRuntimeEventKind::SideEffectPlanned;
        saw_approved |= event.kind == FinanceAccountingRuntimeEventKind::SideEffectApproved;
        assert!(event.replay_ref.starts_with("replay:finance-accounting:"));
    }
    assert!(saw_planned && saw_approved);
}
