use super::*;

#[test]
fn receipt_preflight_validates_source_delivery_verification_and_artifact_bounds() {
    let source = ReceiptSourceReference {
        source_ref: "source-ref".into(),
        source_kind: "payment_intent".into(),
        provider_reference_hash: "provider-hash".into(),
        redaction_class: "reference-only".into(),
    };
    assert!(source.is_visible_reference());

    let receipt = ReceiptRecord {
        receipt_ref: "receipt-ref".into(),
        source_refs: vec![source],
        audience: ReceiptAudience {
            audience_kind: "customer".into(),
            ..Default::default()
        },
        variant: ReceiptVariant {
            variant_kind: "hosted".into(),
            ..Default::default()
        },
        issue_state: "issued".into(),
        lines: vec![ReceiptLine {
            line_ref: "line-ref".into(),
            description_ref: "description-ref".into(),
            quantity_micros: 1_000_000,
            unit_amount_micros: 1_000,
            ..Default::default()
        }],
        totals: ReceiptTotals {
            subtotal_micros: 1_000,
            total_micros: 1_000,
            currency: "USD".into(),
            ..Default::default()
        },
        freshness: ReceiptFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(receipt.has_issue_preconditions(10, 4));

    let stale_receipt = ReceiptRecord {
        freshness: ReceiptFreshness {
            freshness_class: "expired".into(),
            ..Default::default()
        },
        ..receipt
    };
    assert!(!stale_receipt.has_issue_preconditions(10, 4));

    let delivery = ReceiptDeliveryRequest {
        request_ref: "delivery-ref".into(),
        channel: "email_ref".into(),
        destination_ref: "destination-ref".into(),
        approval_ref: Some("approval-ref".into()),
        idempotency_key_hash: "idem-hash".into(),
        ..Default::default()
    };
    assert!(delivery.has_delivery_preconditions());

    let state = ReceiptDeliveryState {
        state: "sent".into(),
        attempt_count: 1,
        provider_message_ref: Some("message-ref".into()),
        ..Default::default()
    };
    assert!(state.is_bounded(3));

    let verification = ReceiptVerificationResult {
        verification_ref: "verification-ref".into(),
        source_linked: true,
        totals_match: true,
        checksum_status: "matched".into(),
        replay_pointer: "replay-ref".into(),
        ..Default::default()
    };
    assert!(verification.is_consistent());

    let correction = ReceiptCorrectionReference {
        correction_ref: "correction-ref".into(),
        correction_kind: "refund".into(),
        source_ref: "source-ref".into(),
        no_side_effect_payload_marker: true,
    };
    assert!(correction.is_boundary_safe());

    let event = ReceiptEventReference {
        event_ref: "event-ref".into(),
        provider_class: "receipt-event".into(),
        event_type: "updated".into(),
        event_timestamp_epoch_ms: 10,
        delivery_id_hash: "delivery-hash".into(),
        webhook_freshness: ReceiptFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        replay_pointer: "replay-ref".into(),
        bounded_result_code: "accepted".into(),
    };
    assert!(event.is_fresh_reference());

    let export = ReceiptAuditExportPlan {
        export_ref: "export-ref".into(),
        scope_ref: "scope-ref".into(),
        format: "json".into(),
        retention_class: "short".into(),
        redaction_profile: "receipt-redacted".into(),
        replay_pointer: "replay-ref".into(),
    };
    assert!(export.is_bounded_plan());

    let artifact = ReceiptArtifactHandle {
        artifact_id: "artifact-ref".into(),
        artifact_type: "hosted".into(),
        hosted_url_metadata_ref: Some("hosted-url-handle".into()),
        checksum: "sha256-redacted".into(),
        expires_at_epoch_ms: 10,
        retention_class: "short".into(),
        redaction_profile: "receipt-redacted".into(),
        access_policy_ref: "policy-ref".into(),
        replay_pointer: "replay-ref".into(),
    };
    assert!(artifact.is_bounded_artifact());

    let raw_artifact = ReceiptArtifactHandle {
        hosted_url_metadata_ref: Some("https://receipt.example/raw".into()),
        ..artifact
    };
    assert!(!raw_artifact.is_bounded_artifact());
}

#[test]
fn entitlement_preflight_validates_isolation_usage_seats_and_proof_bounds() {
    let subject = EntitlementSubject {
        subject_ref: "subject-ref".into(),
        subject_kind: "account".into(),
        redaction_class: "reference-only".into(),
    };
    assert!(subject.is_isolated_reference());

    let resource = EntitlementResource {
        resource_ref: "resource-ref".into(),
        resource_kind: "feature".into(),
        external_resource_ref: Some("external-ref".into()),
    };
    assert!(resource.is_isolated_reference());

    let dimension = EntitlementDimension {
        dimension_ref: "seat-dimension".into(),
        dimension_kind: "seats".into(),
        unit: "count".into(),
    };
    assert!(dimension.is_supported());

    let evidence = EntitlementSourceEvidence {
        source_ref: "source-ref".into(),
        source_kind: "receipt".into(),
        authority_ref: "authority-ref".into(),
        redaction_class: "reference-only".into(),
    };
    assert!(evidence.is_visible_authority());

    let grant = EntitlementGrant {
        grant_ref: "grant-ref".into(),
        subject,
        resource,
        dimensions: vec![dimension.clone()],
        state: CommerceEntitlementState {
            state: "active".into(),
            ..Default::default()
        },
        valid_from_epoch_ms: Some(1),
        valid_until_epoch_ms: Some(10),
        quantity: 1,
        source_evidence: vec![evidence],
        freshness: EntitlementFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(grant.has_grant_preconditions());
    assert!(grant.is_active_at(5));

    let invalid_window = EntitlementGrant {
        valid_from_epoch_ms: Some(10),
        valid_until_epoch_ms: Some(1),
        ..grant.clone()
    };
    assert!(!invalid_window.has_grant_preconditions());

    let seat = EntitlementSeatAssignment {
        assignment_ref: "assignment-ref".into(),
        seat_pool_ref: "seat-pool-ref".into(),
        assignee_ref: "subject-ref".into(),
        quantity: 1,
        assignment_state: "assigned".into(),
        ..Default::default()
    };
    assert!(seat.is_within_limit(5));

    let usage = EntitlementUsageRecord {
        usage_ref: "usage-ref".into(),
        dimension,
        quantity: 5,
        idempotency_key_hash: "idem-hash".into(),
        source_evidence_ref: "source-ref".into(),
        freshness: Some(EntitlementFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(usage.has_recording_preconditions());

    let balance = EntitlementUsageBalance {
        dimension_ref: "seat-dimension".into(),
        balance: 3,
        limit: Some(5),
        ..Default::default()
    };
    assert!(balance.is_within_limit());

    let event = EntitlementEventReference {
        event_ref: "event-ref".into(),
        provider_class: "entitlement-proof".into(),
        event_type: "updated".into(),
        event_timestamp_epoch_ms: 10,
        delivery_id_hash: "delivery-hash".into(),
        webhook_freshness: EntitlementFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        replay_pointer: "replay-ref".into(),
        bounded_result_code: "accepted".into(),
    };
    assert!(event.is_fresh_reference());

    let export = EntitlementProofExportPlan {
        export_ref: "export-ref".into(),
        proof_type: "json".into(),
        scope_ref: "scope-ref".into(),
        retention_class: "short".into(),
        redaction_profile: "entitlement-redacted".into(),
        replay_pointer: "replay-ref".into(),
    };
    assert!(export.is_bounded_plan());

    let artifact = EntitlementArtifactHandle {
        artifact_id: "artifact-ref".into(),
        proof_type: "license".into(),
        checksum: "sha256-redacted".into(),
        expires_at_epoch_ms: 10,
        retention_class: "short".into(),
        redaction_profile: "entitlement-redacted".into(),
        access_policy_ref: "policy-ref".into(),
        replay_pointer: "replay-ref".into(),
    };
    assert!(artifact.is_bounded_artifact());
}
