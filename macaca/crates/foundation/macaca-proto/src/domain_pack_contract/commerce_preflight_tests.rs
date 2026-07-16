use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[test]
fn cart_preflight_rejects_unsafe_pagination_stale_state_and_raw_handoff_data() {
    let envelope = CommercePackCommandEnvelope {
        subject_ref: "cart:subject".into(),
        parameters: BTreeMap::from([("mutation".into(), "line_update".into())]),
        cursor: Some("cursor-1".into()),
        page_size: Some(50),
        idempotency_key: Some("idem-cart-1".into()),
    };
    assert!(envelope.has_bounded_preconditions(100, 8));

    let oversized_page = CommercePackCommandEnvelope {
        page_size: Some(101),
        ..envelope.clone()
    };
    assert!(!oversized_page.has_bounded_preconditions(100, 8));

    let ready_cart = Cart {
        cart_ref: "cart-ref".into(),
        lifecycle_state: "active".into(),
        lines: vec![CartLine {
            line_ref: "line-ref".into(),
            variant_ref: Some("variant-ref".into()),
            quantity: 2,
            validation_state: "valid".into(),
            ..Default::default()
        }],
        estimate: CartEstimate {
            totals: CartTotals {
                subtotal_micros: 200,
                total_micros: 200,
                ..Default::default()
            },
            ..Default::default()
        },
        version_token_hash: "version-current".into(),
        freshness: CartFreshness::default(),
        ..Default::default()
    };
    assert!(ready_cart.is_mutation_ready(10, 4, 99));
    assert!(ready_cart.has_version_conflict("version-old"));
    assert!(!ready_cart.has_version_conflict("version-current"));

    let stale_cart = Cart {
        freshness: CartFreshness {
            stale_flags: BTreeSet::from(["price".into()]),
            ..Default::default()
        },
        ..ready_cart.clone()
    };
    assert!(!stale_cart.is_mutation_ready(10, 4, 99));

    let unsafe_handoff = CartHandoffIntent {
        handoff_ref: "handoff-ref".into(),
        checkout_url_handle: Some("https://checkout.example/raw".into()),
        expires_at_epoch_ms: 10,
        access_policy_ref: "policy-ref".into(),
        no_payment_no_order_marker: true,
        replay_pointer: "replay-ref".into(),
        ..Default::default()
    };
    assert!(!unsafe_handoff.is_boundary_safe());

    let safe_handoff = CartHandoffIntent {
        checkout_url_handle: Some("checkout-url-handle".into()),
        ..unsafe_handoff
    };
    assert!(safe_handoff.is_boundary_safe());

    let artifact = CartArtifactHandle {
        artifact_id: "artifact-ref".into(),
        export_format: "json".into(),
        checksum: "sha256-redacted".into(),
        expires_at_epoch_ms: 10,
        retention_class: "short".into(),
        redaction_profile: "cart-redacted".into(),
    };
    assert!(artifact.is_bounded_export());

    let unbounded_artifact = CartArtifactHandle {
        export_format: "raw-provider-payload".into(),
        ..artifact
    };
    assert!(!unbounded_artifact.is_bounded_export());
}

#[test]
fn catalog_preflight_validates_schema_versions_search_and_export_bounds() {
    let product = CatalogProduct {
        product_ref: "product-ref".into(),
        title_ref: "title-ref".into(),
        localized_content_refs: BTreeMap::from([("en-US".into(), "content-ref".into())]),
        lifecycle_state: "active".into(),
        publication_state: "published".into(),
        product_type_ref: "type-ref".into(),
        provider_version_token: "version-ref".into(),
        ..Default::default()
    };
    assert!(product.has_required_schema_fields());

    let variant = CatalogVariant {
        variant_ref: "variant-ref".into(),
        product_ref: "product-ref".into(),
        sku_ref: "sku-ref".into(),
        option_values: BTreeMap::from([("size".into(), "medium".into())]),
        purchasable: true,
        provider_version_token: "variant-version".into(),
        ..Default::default()
    };
    assert!(variant.has_required_schema_fields());

    let search = CatalogSearchRequest {
        query_ref: "query-ref".into(),
        filters: BTreeMap::from([("status".into(), "active".into())]),
        facets: BTreeSet::from(["brand".into()]),
        sort_ref: Some("relevance".into()),
        max_results: 25,
        ..Default::default()
    };
    assert!(search.has_portable_preconditions(50, 4, 4));

    let unsafe_search = CatalogSearchRequest {
        filters: BTreeMap::from([("provider_dsl".into(), "https://vendor.example/raw".into())]),
        ..search
    };
    assert!(!unsafe_search.has_portable_preconditions(50, 4, 4));

    let mutation = CatalogMutationPlan {
        plan_ref: "plan-ref".into(),
        target_ref: "product-ref".into(),
        mutation_kind: "publish".into(),
        required_approval: true,
        idempotency_key: "idem-catalog".into(),
        ..Default::default()
    };
    assert!(mutation.requires_approval());
    assert!(mutation.has_execution_preconditions());

    let stale = CatalogFreshness {
        source_timestamp_epoch_ms: 1,
        freshness_class: "stale".into(),
        stale_reason: Some("provider_snapshot_expired".into()),
        ..Default::default()
    };
    assert!(stale.has_stale_data());

    let artifact = CatalogArtifactHandle {
        artifact_id: "artifact-ref".into(),
        export_format: "ndjson".into(),
        checksum: "sha256-redacted".into(),
        expires_at_epoch_ms: 10,
        retention_class: "short".into(),
        redaction_profile: "catalog-redacted".into(),
    };
    assert!(artifact.is_bounded_export());
}

#[test]
fn order_preflight_validates_lifecycle_fulfillment_cancellation_and_audit_bounds() {
    let order = OrderRecord {
        order_ref: "order-ref".into(),
        lifecycle_state: OrderLifecycleState {
            state: "created".into(),
            ..Default::default()
        },
        lines: vec![OrderLine {
            line_ref: "line-ref".into(),
            variant_ref: Some("variant-ref".into()),
            quantity: 2,
            price_snapshot_micros: 500,
            tax_micros: 50,
            ..Default::default()
        }],
        totals: OrderTotals {
            subtotal_micros: 1_000,
            tax_micros: 100,
            total_micros: 1_100,
            ..Default::default()
        },
        version_token_hash: "order-version".into(),
        freshness: OrderFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(order.is_lifecycle_ready(10, 4, 99));
    assert!(order.has_version_conflict("old-version"));
    assert!(!order.has_version_conflict("order-version"));

    let stale_order = OrderRecord {
        freshness: OrderFreshness {
            freshness_class: "stale".into(),
            ..Default::default()
        },
        ..order.clone()
    };
    assert!(!stale_order.is_lifecycle_ready(10, 4, 99));

    let transition = OrderLifecycleTransitionPlan {
        plan_ref: "transition-ref".into(),
        from_state: "created".into(),
        to_state: "cancelled".into(),
        requires_approval: true,
        ..Default::default()
    };
    assert!(transition.has_valid_transition());

    let fulfillment = FulfillmentIntent {
        intent_ref: "fulfillment-ref".into(),
        intent_kind: "shipment".into(),
        line_allocations: BTreeMap::from([("line-ref".into(), 2)]),
        tracking_reference_handle: Some("tracking-handle".into()),
        carrier_handoff_boundary: true,
        ..Default::default()
    };
    assert!(fulfillment.is_boundary_safe());

    let raw_tracking = FulfillmentIntent {
        tracking_reference_handle: Some("https://carrier.example/raw".into()),
        ..fulfillment
    };
    assert!(!raw_tracking.is_boundary_safe());

    let cancellation = OrderCancellationPlan {
        plan_ref: "cancel-plan".into(),
        order_ref: "order-ref".into(),
        reason_ref: "reason-ref".into(),
        provider_supported: true,
        requires_approval: true,
        ..Default::default()
    };
    assert!(cancellation.is_eligible());

    let export_plan = OrderAuditExportPlan {
        export_ref: "export-ref".into(),
        scope_ref: "scope-ref".into(),
        format: "json".into(),
        redaction_profile: "order-redacted".into(),
        retention_class: "short".into(),
        replay_pointer: "replay-ref".into(),
    };
    assert!(export_plan.is_bounded_plan());

    let artifact = OrderArtifactHandle {
        artifact_id: "artifact-ref".into(),
        export_format: "csv".into(),
        checksum: "sha256-redacted".into(),
        expires_at_epoch_ms: 10,
        retention_class: "short".into(),
        access_policy_ref: "policy-ref".into(),
    };
    assert!(artifact.is_bounded_export());
}

#[test]
fn payment_intent_preflight_rejects_raw_credentials_and_unbounded_gateway_data() {
    let method = PaymentMethodReference {
        token_ref: "token-ref".into(),
        method_type: "card".into(),
        region_support: BTreeSet::from(["US".into()]),
        raw_credential_rejected: false,
        ..Default::default()
    };
    assert!(method.has_safe_token_reference());

    let raw_method = PaymentMethodReference {
        token_ref: "4111111111111111".into(),
        raw_credential_rejected: true,
        ..method.clone()
    };
    assert!(!raw_method.has_safe_token_reference());

    let plan = PaymentIntentPlan {
        plan_ref: "plan-ref".into(),
        amount_micros: 1_000,
        currency: "USD".into(),
        capture_mode: "manual".into(),
        merchant_account_ref: "merchant-ref".into(),
        payment_method: method,
        idempotency_key_hash: "idem-hash".into(),
        ..Default::default()
    };
    assert!(plan.has_execution_preconditions());

    let action = PaymentActionRequirement {
        action_ref: "action-ref".into(),
        action_type: "redirect".into(),
        redirect_handle: Some("redirect-handle".into()),
        expires_at_epoch_ms: 10,
        ..Default::default()
    };
    assert!(action.is_handle_only());

    let raw_action = PaymentActionRequirement {
        redirect_handle: Some("https://gateway.example/client-secret".into()),
        ..action.clone()
    };
    assert!(!raw_action.is_handle_only());

    let record = PaymentIntentRecord {
        payment_intent_ref: "intent-ref".into(),
        amount_micros: 1_000,
        currency: "USD".into(),
        capture_mode: "manual".into(),
        state: "requires_action".into(),
        action_requirements: vec![action],
        freshness: PaymentIntentFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(record.has_valid_state());

    let authorization = PaymentAuthorization {
        authorization_ref: "auth-ref".into(),
        amount_micros: 1_000,
        currency: "USD".into(),
        expires_at_epoch_ms: 10,
        provider_reference_hash: "provider-hash".into(),
        side_effect_evidence_ref: "evidence-ref".into(),
    };
    assert!(authorization.is_unexpired_reference());

    let capture = PaymentCapture {
        capture_ref: "capture-ref".into(),
        amount_micros: 500,
        currency: "USD".into(),
        partial_capture: true,
        provider_reference_hash: "provider-hash".into(),
        side_effect_evidence_ref: "evidence-ref".into(),
    };
    assert!(capture.is_amount_allowed(1_000));
    assert!(!capture.is_amount_allowed(250));

    let cancellation = PaymentCancellation {
        cancellation_ref: "cancel-ref".into(),
        reason_ref: "reason-ref".into(),
        provider_reference_hash: "provider-hash".into(),
        side_effect_evidence_ref: "evidence-ref".into(),
    };
    assert!(cancellation.is_boundary_safe());

    let event = PaymentIntentEventReference {
        event_ref: "event-ref".into(),
        provider_class: "payment-event".into(),
        event_type: "updated".into(),
        event_timestamp_epoch_ms: 10,
        delivery_id_hash: "delivery-hash".into(),
        webhook_freshness: PaymentIntentFreshness {
            freshness_class: "fresh".into(),
            ..Default::default()
        },
        replay_pointer: "replay-ref".into(),
        bounded_result_code: "accepted".into(),
    };
    assert!(event.is_fresh_reference());

    let export_plan = PaymentIntentAuditExportPlan {
        export_ref: "export-ref".into(),
        scope_ref: "scope-ref".into(),
        format: "json".into(),
        redaction_profile: "payment-redacted".into(),
    };
    assert!(export_plan.is_bounded_plan());

    let artifact = PaymentIntentArtifactHandle {
        artifact_id: "artifact-ref".into(),
        export_format: "ndjson".into(),
        checksum: "sha256-redacted".into(),
        expires_at_epoch_ms: 10,
        retention_class: "short".into(),
        redaction_profile: "payment-redacted".into(),
        access_policy_ref: "policy-ref".into(),
    };
    assert!(artifact.is_bounded_export());
}
