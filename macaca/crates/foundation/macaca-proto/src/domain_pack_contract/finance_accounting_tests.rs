use std::collections::BTreeSet;

use super::finance_accounting::*;
use super::model::DomainPackProviderCapabilityState;

// Accounting validation tests stay at the provider-neutral contract layer. They
// prove preflight DTO rules without constructing ERP, bank-feed, tax, payroll,
// invoice, payment, or portfolio providers.

#[test]
fn accounting_journal_preflight_validates_side_effect_safety() {
    let active_accounts = BTreeSet::from(["cash".into(), "revenue".into()]);
    let required_dimensions = BTreeSet::from(["department".into()]);
    let plan = JournalEntryPlan {
        plan_ref: "plan".into(),
        period_ref: "period".into(),
        idempotency_key: "idem-journal".into(),
        lines: vec![
            JournalLine {
                account_ref: "cash".into(),
                debit_micros: 10_000,
                currency: "USD".into(),
                dimensions: vec![AccountingDimension {
                    dimension_ref: "dim".into(),
                    dimension_kind: "department".into(),
                    value_ref: "engineering".into(),
                }],
                tax_code: Some(TaxCodeReference {
                    tax_code_ref: "tax-ref".into(),
                    jurisdiction_ref: "us-ca".into(),
                }),
                ..Default::default()
            },
            JournalLine {
                account_ref: "revenue".into(),
                credit_micros: 10_000,
                currency: "USD".into(),
                dimensions: vec![AccountingDimension {
                    dimension_ref: "dim".into(),
                    dimension_kind: "department".into(),
                    value_ref: "engineering".into(),
                }],
                ..Default::default()
            },
        ],
    };

    assert!(plan.balances());
    assert!(plan.balances_by_currency());
    assert!(plan.has_idempotency_key(64));
    assert!(plan.has_required_dimensions(&required_dimensions));
    assert!(plan.references_only_active_accounts(&active_accounts));
    assert!(plan.has_valid_reference_shapes());
}

#[test]
fn accounting_journal_preflight_rejects_invalid_shapes() {
    let plan = JournalEntryPlan {
        idempotency_key: "idem".into(),
        lines: vec![
            JournalLine {
                account_ref: "cash".into(),
                debit_micros: 10_000,
                currency: "USD".into(),
                ..Default::default()
            },
            JournalLine {
                account_ref: "revenue".into(),
                credit_micros: 10_000,
                currency: "EUR".into(),
                tax_code: Some(TaxCodeReference {
                    tax_code_ref: "tax\nraw".into(),
                    jurisdiction_ref: "us".into(),
                }),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    assert!(!plan.balances_by_currency());
    assert!(!plan.has_required_dimensions(&BTreeSet::from(["department".into()])));
    assert!(!plan.references_only_active_accounts(&BTreeSet::from(["cash".into()])));
    assert!(!plan.has_valid_reference_shapes());
}

#[test]
fn accounting_period_and_provider_preflight_is_descriptor_driven() {
    let open_period = AccountingPeriod {
        close_state: AccountingPeriodCloseState {
            state: "open".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(open_period.allows_posting());

    let locked_period = AccountingPeriod {
        lock: AccountingPeriodLock {
            locked: true,
            lock_reason: Some("close_in_progress".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!locked_period.allows_posting());

    let capability = AccountingProviderCapability {
        provider_class: "mock".into(),
        supported_commands: BTreeSet::from(["accounting.post_journal".into()]),
        supported_reports: BTreeSet::from(["trial_balance".into()]),
        write_support: true,
        state: DomainPackProviderCapabilityState::Preview,
    };
    assert!(capability.allows_write_command("accounting.post_journal"));
    assert!(capability.supports_report("trial_balance"));

    let unavailable = AccountingProviderCapability {
        state: DomainPackProviderCapabilityState::Unavailable,
        ..capability
    };
    assert!(!unavailable.allows_write_command("accounting.post_journal"));
    assert!(!unavailable.supports_report("trial_balance"));
}

#[test]
fn accounting_request_bounds_are_trace_safe() {
    let request = AccountingReportRequest {
        request_ref: "request".into(),
        basis: "accrual".into(),
        period_range: "2026-Q2".into(),
        currency: "USD".into(),
        dimensions: vec![AccountingDimension {
            dimension_ref: "dim".into(),
            dimension_kind: "department".into(),
            value_ref: "engineering".into(),
        }],
        pagination: AccountingPaginationMetadata {
            next_cursor: Some("cursor".into()),
            page_size: 50,
            truncated: false,
        },
        async_metadata: Some(AccountingAsyncMetadata {
            job_ref: "job".into(),
            state: "completed".into(),
            submitted_at_epoch_ms: 1,
            result_artifact_ref: Some("artifact".into()),
            replay_pointer: "replay".into(),
        }),
    };
    assert!(request.is_bounded(4, 100));

    let unbounded = AccountingReportRequest {
        pagination: AccountingPaginationMetadata {
            next_cursor: Some("cursor\nraw".into()),
            page_size: 10_000,
            truncated: true,
        },
        ..request
    };
    assert!(!unbounded.is_bounded(4, 100));
}

#[test]
fn accounting_bounded_command_spec_covers_ledger_report_and_export() {
    let spec = AccountingBoundedCommandSpec::default();
    let execution = AccountingExecutionControl {
        timeout_ms: 30_000,
        cancellation_ref: Some("cancel-ref".into()),
        replay_pointer: "replay".into(),
    };
    let output = AccountingOutputBound {
        row_count: 50,
        estimated_bytes: 4_096,
    };
    let pagination = AccountingPaginationMetadata {
        next_cursor: Some("cursor".into()),
        page_size: 50,
        truncated: false,
    };

    assert!(spec.validates_ledger_command(&pagination, &execution, &output));

    let report = AccountingReportRequest {
        request_ref: "request".into(),
        basis: "accrual".into(),
        period_range: "2026-Q2".into(),
        dimensions: vec![AccountingDimension {
            dimension_ref: "dim".into(),
            dimension_kind: "department".into(),
            value_ref: "engineering".into(),
        }],
        currency: "USD".into(),
        pagination,
        async_metadata: Some(AccountingAsyncMetadata {
            job_ref: "job".into(),
            state: "queued".into(),
            submitted_at_epoch_ms: 1,
            result_artifact_ref: Some("artifact".into()),
            replay_pointer: "report-replay".into(),
        }),
    };
    assert!(spec.validates_report_command(&report, &execution, &output));

    let export = AuditExportPlan {
        plan_ref: "export-plan".into(),
        export_format: "jsonl".into(),
        retention_policy: "short_retention".into(),
        redaction: AccountingRedactionPolicy {
            policy_ref: "redaction".into(),
            redacted_fields: BTreeSet::from(["account_number".into(), "tax_identifier".into()]),
            export_profile: "metadata_only".into(),
        },
    };
    let artifact = AccountingArtifactHandle {
        artifact_id: "artifact".into(),
        artifact_kind: "audit_export".into(),
        access_policy: "scoped".into(),
        expires_at_epoch_ms: 1,
    };
    assert!(spec.validates_export_command(&export, &artifact, &execution, &output));
}

#[test]
fn accounting_bounded_command_spec_rejects_unbounded_controls() {
    let spec = AccountingBoundedCommandSpec::default();
    let unsafe_execution = AccountingExecutionControl {
        timeout_ms: spec.max_timeout_ms + 1,
        cancellation_ref: None,
        replay_pointer: "replay".into(),
    };
    let unsafe_output = AccountingOutputBound {
        row_count: spec.max_output_rows + 1,
        estimated_bytes: spec.max_output_bytes + 1,
    };
    let unsafe_pagination = AccountingPaginationMetadata {
        next_cursor: Some("cursor\nraw".into()),
        page_size: spec.max_page_size + 1,
        truncated: true,
    };

    assert!(!spec.validates_ledger_command(&unsafe_pagination, &unsafe_execution, &unsafe_output));

    let unsafe_report = AccountingReportRequest {
        request_ref: "request".into(),
        basis: "accrual".into(),
        period_range: "2026-Q2".into(),
        currency: "USD".into(),
        pagination: unsafe_pagination,
        async_metadata: Some(AccountingAsyncMetadata {
            job_ref: "job\nraw".into(),
            state: "queued".into(),
            submitted_at_epoch_ms: 1,
            result_artifact_ref: Some("artifact".into()),
            replay_pointer: "report-replay".into(),
        }),
        ..Default::default()
    };
    assert!(!spec.validates_report_command(&unsafe_report, &unsafe_execution, &unsafe_output));

    let unsafe_export = AuditExportPlan {
        plan_ref: "export-plan".into(),
        export_format: "jsonl".into(),
        retention_policy: "retention\nraw".into(),
        redaction: AccountingRedactionPolicy::default(),
    };
    let unsafe_artifact = AccountingArtifactHandle {
        artifact_id: "artifact\nraw".into(),
        artifact_kind: "audit_export".into(),
        access_policy: "scoped".into(),
        expires_at_epoch_ms: 0,
    };
    assert!(!spec.validates_export_command(
        &unsafe_export,
        &unsafe_artifact,
        &unsafe_execution,
        &unsafe_output
    ));
}

#[test]
fn accounting_declaration_spec_validates_allowed_scopes() {
    let spec = AccountingDeclarationSpec;
    for permission_scope in spec.allowed_scopes() {
        let scope = AccountingScope {
            tenant_scope: "tenant".into(),
            entity_ref: "entity".into(),
            ledger_book_ref: "book".into(),
            permission_scope: (*permission_scope).into(),
        };
        assert!(spec.validate_scope(&scope).is_ok());
    }
}

#[test]
fn accounting_declaration_spec_rejects_unknown_or_unbounded_scopes() {
    let spec = AccountingDeclarationSpec;
    let unknown_scope = AccountingScope {
        tenant_scope: "tenant".into(),
        entity_ref: "entity".into(),
        ledger_book_ref: "book".into(),
        permission_scope: "finance.accounting.admin".into(),
    };
    assert!(spec.validate_scope(&unknown_scope).is_err());

    let unbounded_ref = AccountingScope {
        tenant_scope: "tenant\nraw".into(),
        entity_ref: "entity".into(),
        ledger_book_ref: "book".into(),
        permission_scope: "finance.accounting.read".into(),
    };
    assert!(spec.validate_scope(&unbounded_ref).is_err());
}

#[test]
fn accounting_preflight_spec_accepts_policy_approved_commands() {
    let spec = AccountingCommandPreflightSpec;
    for command in [
        "accounting.get_ledger_entries",
        "accounting.post_journal",
        "accounting.generate_trial_balance",
        "accounting.audit_export_request",
    ] {
        let preflight = AccountingCommandPreflight::allowed(command);
        assert!(spec.evaluate(&preflight).is_ok(), "{command}");
    }
}

#[test]
fn accounting_preflight_spec_returns_typed_pre_provider_rejections() {
    let spec = AccountingCommandPreflightSpec;

    let mut denied = AccountingCommandPreflight::allowed("accounting.post_journal");
    denied.policy.allowed = false;
    assert_eq!(
        spec.evaluate(&denied).unwrap_err().status,
        AccountingResultStatus::Denied
    );

    let mut missing_approval = AccountingCommandPreflight::allowed("accounting.post_journal");
    missing_approval.approval = None;
    assert_eq!(
        spec.evaluate(&missing_approval).unwrap_err().reason_code,
        "approval_required"
    );

    let mut quota = AccountingCommandPreflight::allowed("accounting.generate_trial_balance");
    quota.resources.async_job_slots = 0;
    assert_eq!(
        spec.evaluate(&quota).unwrap_err().status,
        AccountingResultStatus::QuotaExceeded
    );

    let mut unavailable = AccountingCommandPreflight::allowed("accounting.get_account");
    unavailable.entitlement.provider_access = false;
    assert_eq!(
        spec.evaluate(&unavailable).unwrap_err().status,
        AccountingResultStatus::Unavailable
    );

    let mut unsupported = AccountingCommandPreflight::allowed("accounting.audit_export_request");
    unsupported.entitlement.export_support = false;
    assert_eq!(
        spec.evaluate(&unsupported).unwrap_err().status,
        AccountingResultStatus::Unsupported
    );

    let mut conflict = AccountingCommandPreflight::allowed("accounting.post_journal");
    conflict.consistency.conflict_free = false;
    assert_eq!(
        spec.evaluate(&conflict).unwrap_err().status,
        AccountingResultStatus::Conflict
    );

    let mut stale = AccountingCommandPreflight::allowed("accounting.get_ledger_entries");
    stale.consistency.freshness_current = false;
    assert_eq!(
        spec.evaluate(&stale).unwrap_err().status,
        AccountingResultStatus::StaleData
    );
}

#[test]
fn accounting_preflight_requires_approval_for_all_side_effect_commands() {
    let spec = AccountingCommandPreflightSpec;
    for command in [
        "accounting.account_request",
        "accounting.post_journal",
        "accounting.import_statement_lines",
        "accounting.reconciliation_request",
        "accounting.audit_export_request",
    ] {
        let mut preflight = AccountingCommandPreflight::allowed(command);
        preflight.approval = None;

        let rejection = spec.evaluate(&preflight).unwrap_err();
        assert_eq!(
            rejection.status,
            AccountingResultStatus::Denied,
            "{command}"
        );
        assert_eq!(rejection.reason_code, "approval_required", "{command}");
        assert!(requires_approval(command));
    }
}

#[test]
fn accounting_preflight_checks_resource_families_before_provider_dispatch() {
    let ledger = AccountingResourceRequirement::for_command("accounting.get_ledger_entries");
    assert_eq!(ledger.provider_call_units, 1);
    assert_eq!(ledger.ledger_page_units, 1);
    assert_eq!(ledger.network_quota_units, 1);

    let report = AccountingResourceRequirement::for_command("accounting.generate_cash_flow");
    assert_eq!(report.provider_call_units, 1);
    assert_eq!(report.report_generation_units, 1);
    assert_eq!(report.async_job_slots, 1);

    let export = AccountingResourceRequirement::for_command("accounting.audit_export_request");
    assert_eq!(export.provider_call_units, 1);
    assert_eq!(export.export_bytes, 1);
    assert_eq!(export.retained_artifact_units, 1);
    assert_eq!(export.storage_bytes, 1);
}

#[test]
fn accounting_preflight_checks_write_report_export_and_entity_entitlements() {
    let spec = AccountingCommandPreflightSpec;

    let mut write_missing = AccountingCommandPreflight::allowed("accounting.post_journal");
    write_missing.entitlement.write_support = false;
    assert_eq!(
        spec.evaluate(&write_missing).unwrap_err().reason_code,
        "write_support_missing"
    );

    let mut report_missing =
        AccountingCommandPreflight::allowed("accounting.generate_balance_sheet");
    report_missing.entitlement.report_support = false;
    assert_eq!(
        spec.evaluate(&report_missing).unwrap_err().reason_code,
        "report_support_missing"
    );

    let mut export_missing = AccountingCommandPreflight::allowed("accounting.audit_export_request");
    export_missing.entitlement.export_support = false;
    assert_eq!(
        spec.evaluate(&export_missing).unwrap_err().reason_code,
        "export_support_missing"
    );

    let mut entity_missing = AccountingCommandPreflight::allowed("accounting.get_account");
    entity_missing.entitlement.entity_access = false;
    assert_eq!(
        spec.evaluate(&entity_missing).unwrap_err().reason_code,
        "entity_denied"
    );
}
