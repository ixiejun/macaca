use std::collections::BTreeMap;

use super::knowledge_summarization::{
    SummaryRequest, SummarySource, KNOWLEDGE_SUMMARIZATION_COMMANDS,
};
use super::knowledge_summarization_preflight::{
    SummarizationAdmissionEvidence, SummarizationDispatchPreflight,
};
use super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

#[test]
fn summarization_admission_maps_every_command_to_its_declared_permission() {
    let gate = SummarizationDispatchPreflight::new(["summarization.compress_context"]);
    for command in KNOWLEDGE_SUMMARIZATION_COMMANDS {
        let request = valid_request();
        let mut dispatched = false;
        let result = gate.dispatch_after_preflight(
            if requires_sources(command) {
                Some(&request)
            } else {
                None
            },
            &preflight(command),
            &evidence(),
            || {
                dispatched = true;
                "summary-handle"
            },
        );
        assert_eq!(result, Ok("summary-handle"), "{command}");
        assert!(dispatched, "{command}");
    }
}

#[test]
fn summarization_admission_rejects_each_host_gate_before_provider_dispatch() {
    let gate = SummarizationDispatchPreflight::new(["summarization.compress_context"]);
    let request = valid_request();
    let preflight = preflight("summarization.compress_context");
    for rejected in rejected_evidence_cases() {
        let mut dispatched = false;
        assert!(gate
            .dispatch_after_preflight(Some(&request), &preflight, &rejected, || {
                dispatched = true
            })
            .is_err());
        assert!(!dispatched);
    }
}

#[test]
fn summarization_admission_returns_structured_rejections() {
    let gate = SummarizationDispatchPreflight::new(["summarization.compress_context"]);
    let request = valid_request();
    let invalid_scope = DomainPackCommandPreflight {
        requested_scope: "summarization.unknown".into(),
        ..preflight("summarization.summarize")
    };
    assert_eq!(
        gate.evaluate(Some(&request), &invalid_scope, &evidence())
            .unwrap_err()
            .reason_code,
        "permission_not_declared"
    );
    let quota = SummarizationAdmissionEvidence {
        provider_quota_available: false,
        ..evidence()
    };
    assert_eq!(
        gate.evaluate(
            Some(&request),
            &preflight("summarization.summarize"),
            &quota
        )
        .unwrap_err()
        .status,
        DomainPackPreflightStatus::QuotaExceeded
    );
    let unavailable = SummarizationAdmissionEvidence {
        citation_support_available: false,
        ..evidence()
    };
    assert_eq!(
        gate.evaluate(
            Some(&request),
            &preflight("summarization.summarize_with_citations"),
            &unavailable,
        )
        .unwrap_err()
        .status,
        DomainPackPreflightStatus::Unavailable
    );
    let mut approval = preflight("summarization.compress_context");
    approval.approval = None;
    assert_eq!(
        gate.evaluate(Some(&request), &approval, &evidence())
            .unwrap_err()
            .reason_code,
        "approval_required"
    );
}

#[test]
fn summarization_admission_rejects_unknown_or_unbounded_source_references() {
    let gate = SummarizationDispatchPreflight::new([] as [&str; 0]);
    let invalid = SummaryRequest {
        sources: vec![SummarySource {
            source_kind: "raw_document".into(),
            ..valid_request().sources.remove(0)
        }],
        ..valid_request()
    };
    let error = gate
        .evaluate(
            Some(&invalid),
            &preflight("summarization.summarize"),
            &evidence(),
        )
        .unwrap_err();
    assert_eq!(error.reason_code, "summarization_source_request_invalid");
}

fn requires_sources(command: &str) -> bool {
    !matches!(
        command,
        "summarization.inspect_provider"
            | "summarization.inspect_summary_evidence"
            | "summarization.refine_summary"
            | "summarization.compare_summaries"
            | "summarization.evaluate_summary"
    )
}

fn valid_request() -> SummaryRequest {
    SummaryRequest {
        request_id: "summary:request".into(),
        sources: vec![SummarySource {
            source_ref: "source:one".into(),
            source_kind: "document".into(),
            revision: "revision:one".into(),
            sensitivity: "internal".into(),
        }],
        mode: "extractive".into(),
        target_tokens: 256,
        language: Some("en".into()),
    }
}

fn preflight(command: &str) -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: command.into(),
        requested_scope: scope(command).into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:summary".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:summary".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "allowed".into(),
        },
        required_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
        reserved_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
    }
}

fn scope(command: &str) -> &'static str {
    match command {
        "summarization.plan" | "summarization.validate_request" => "summarization.plan",
        "summarization.summarize" | "summarization.summarize_many" => "summarization.run",
        "summarization.summarize_with_citations" => "summarization.citations",
        "summarization.summarize_conversation" => "summarization.conversation",
        "summarization.compress_context" => "summarization.context.compress",
        "summarization.refine_summary" => "summarization.refine",
        "summarization.compare_summaries" => "summarization.compare",
        "summarization.evaluate_summary" => "summarization.evaluate",
        "summarization.inspect_summary_evidence" => "summarization.evidence.read",
        _ => "summarization.provider.inspect",
    }
}

fn evidence() -> SummarizationAdmissionEvidence {
    SummarizationAdmissionEvidence {
        source_handles_accessible: true,
        source_kinds_supported: true,
        mode_allowed: true,
        output_schema_allowed: true,
        target_length_allowed: true,
        language_allowed: true,
        evidence_allowed: true,
        quote_allowed: true,
        freshness_allowed: true,
        sensitivity_allowed: true,
        compression_retention_allowed: true,
        quality_threshold_met: true,
        chunk_limit_available: true,
        streaming_available: true,
        timeout_available: true,
        memory_available: true,
        storage_available: true,
        network_allowed: true,
        provider_quota_available: true,
        evaluation_budget_available: true,
        snapshot_capacity_available: true,
        citation_support_available: true,
        compression_support_available: true,
        evaluation_support_available: true,
    }
}

fn rejected_evidence_cases() -> Vec<SummarizationAdmissionEvidence> {
    let base = evidence();
    vec![
        SummarizationAdmissionEvidence {
            source_handles_accessible: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            source_kinds_supported: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            mode_allowed: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            target_length_allowed: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            sensitivity_allowed: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            chunk_limit_available: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            provider_quota_available: false,
            ..base.clone()
        },
        SummarizationAdmissionEvidence {
            compression_support_available: false,
            ..base
        },
    ]
}
