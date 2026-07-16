use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::commerce_cart::*;
use super::commerce_catalog::*;
use super::commerce_entitlement::*;
use super::commerce_order::*;
use super::commerce_payment_intent::*;
use super::commerce_receipt::*;
use super::*;

// Commerce tests validate provider-neutral descriptor contracts only. They do
// not contact storefronts, cart engines, payment gateways, receipt services, or
// entitlement providers, and fixtures use synthetic handles instead of buyer
// PII, payment credentials, webhook bodies, checkout URLs, or provider payloads.

#[test]
fn commerce_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            commerce_catalog_pack_definition(),
            COMMERCE_CATALOG_PACK_ID,
            COMMERCE_CATALOG_SERVICE_ID,
            COMMERCE_CATALOG_COMMANDS,
            "commerce_catalog_provider_not_installed",
            "catalog-search",
            "catalog.search_catalog",
        ),
        (
            commerce_cart_pack_definition(),
            COMMERCE_CART_PACK_ID,
            COMMERCE_CART_SERVICE_ID,
            COMMERCE_CART_COMMANDS,
            "commerce_cart_provider_not_installed",
            "cart-handoff",
            "cart.plan_handoff",
        ),
        (
            commerce_order_pack_definition(),
            COMMERCE_ORDER_PACK_ID,
            COMMERCE_ORDER_SERVICE_ID,
            COMMERCE_ORDER_COMMANDS,
            "commerce_order_provider_not_installed",
            "order-lifecycle",
            "order.plan_state_transition",
        ),
        (
            commerce_payment_intent_pack_definition(),
            COMMERCE_PAYMENT_INTENT_PACK_ID,
            COMMERCE_PAYMENT_INTENT_SERVICE_ID,
            COMMERCE_PAYMENT_INTENT_COMMANDS,
            "commerce_payment_intent_provider_not_installed",
            "payment-action",
            "payment_intent.inspect_action",
        ),
        (
            commerce_receipt_pack_definition(),
            COMMERCE_RECEIPT_PACK_ID,
            COMMERCE_RECEIPT_SERVICE_ID,
            COMMERCE_RECEIPT_COMMANDS,
            "commerce_receipt_provider_not_installed",
            "receipt-delivery",
            "receipt.delivery_request",
        ),
        (
            commerce_entitlement_pack_definition(),
            COMMERCE_ENTITLEMENT_PACK_ID,
            COMMERCE_ENTITLEMENT_SERVICE_ID,
            COMMERCE_ENTITLEMENT_COMMANDS,
            "commerce_entitlement_provider_not_installed",
            "entitlement-usage",
            "entitlement.record_usage",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.commerce.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/commerce"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("commerce descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_commerce_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let catalog = find_pack(&definitions, COMMERCE_CATALOG_PACK_ID);
    let cart = find_pack(&definitions, COMMERCE_CART_PACK_ID);
    let order = find_pack(&definitions, COMMERCE_ORDER_PACK_ID);
    let payment = find_pack(&definitions, COMMERCE_PAYMENT_INTENT_PACK_ID);
    let receipt = find_pack(&definitions, COMMERCE_RECEIPT_PACK_ID);
    let entitlement = find_pack(&definitions, COMMERCE_ENTITLEMENT_PACK_ID);

    assert_eq!(
        catalog
            .metadata
            .provider_descriptors
            .get("catalog-mutation")
            .and_then(|descriptor| descriptor.metadata.get("publish"))
            .map(String::as_str),
        Some("approval_required")
    );
    assert_eq!(
        cart.metadata
            .provider_descriptors
            .get("cart-handoff")
            .and_then(|descriptor| descriptor.metadata.get("payment_execution"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        order
            .metadata
            .provider_descriptors
            .get("fulfillment-intent")
            .and_then(|descriptor| descriptor.metadata.get("carrier_execution"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        payment
            .metadata
            .provider_descriptors
            .get("payment-action")
            .and_then(|descriptor| descriptor.metadata.get("client_secret"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        receipt
            .metadata
            .provider_descriptors
            .get("receipt-delivery")
            .and_then(|descriptor| descriptor.metadata.get("communication_workflow"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        entitlement
            .metadata
            .provider_descriptors
            .get("entitlement-usage")
            .and_then(|descriptor| descriptor.metadata.get("application_feature_gate"))
            .map(String::as_str),
        Some("false")
    );
}

#[test]
fn commerce_command_and_result_dtos_are_serde_compatible() {
    let envelope = CommercePackCommandEnvelope {
        subject_ref: "commerce:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(20),
        idempotency_key: Some("idem-commerce".into()),
    };

    let values = [
        serde_json::to_value(CatalogSearchCatalogCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(CartPlanHandoffCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(OrderPlanStateTransitionCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(PaymentIntentInspectActionCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(ReceiptDeliveryRequestCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(CommerceEntitlementRecordUsageCommand { request: envelope }).unwrap(),
        serde_json::to_value(CatalogResultEnvelope::<CatalogProduct> {
            status: CatalogResultStatus::Unsupported,
            data: None,
            page: None,
            error: Some(CommercePackError {
                code: "unsupported_filter".into(),
                message: "synthetic unsupported filter".into(),
                retryable: false,
                trace_safe_detail: Some("portable_filter_required".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(CartResultEnvelope::<Cart> {
            status: CartResultStatus::StaleData,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(OrderResultEnvelope::<OrderRecord> {
            status: OrderResultStatus::LifecycleInvalid,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(PaymentIntentResultEnvelope::<PaymentIntentRecord> {
            status: PaymentIntentResultStatus::RawCredentialRejected,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(ReceiptResultEnvelope::<ReceiptRecord> {
            status: ReceiptResultStatus::ArtifactRedacted,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(EntitlementResultEnvelope::<EntitlementGrant> {
            status: EntitlementResultStatus::ProofRedacted,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn commerce_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&commerce_catalog_descriptor_hashes()),
        hash_values(&commerce_cart_descriptor_hashes()),
        hash_values(&commerce_order_descriptor_hashes()),
        hash_values(&commerce_payment_intent_descriptor_hashes()),
        hash_values(&commerce_receipt_descriptor_hashes()),
        hash_values(&commerce_entitlement_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 8);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn commerce_validation_helpers_are_provider_neutral() {
    let search = CatalogSearchRequest {
        filters: BTreeMap::from([("status".into(), "active".into())]),
        max_results: 25,
        ..Default::default()
    };
    assert!(search.is_bounded(50, 2));

    let mutation = CatalogMutationPlan {
        mutation_kind: "publish".into(),
        ..Default::default()
    };
    assert!(mutation.requires_approval());

    let cart = Cart {
        lines: vec![CartLine::default()],
        validation_issues: vec![CartValidationIssue::default()],
        ..Default::default()
    };
    assert!(cart.is_bounded(2, 2));

    let cart_totals = CartTotals {
        subtotal_micros: 100,
        total_micros: 100,
        ..Default::default()
    };
    assert!(cart_totals.totals_match());

    let order_totals = OrderTotals {
        subtotal_micros: 100,
        total_micros: 100,
        ..Default::default()
    };
    assert!(order_totals.totals_match());

    let payment_method = PaymentMethodReference {
        token_ref: "token".into(),
        raw_credential_rejected: false,
        ..Default::default()
    };
    assert!(payment_method.is_tokenized_only());

    let receipt_totals = ReceiptTotals {
        subtotal_micros: 100,
        total_micros: 100,
        ..Default::default()
    };
    assert!(receipt_totals.totals_match());

    let grant = EntitlementGrant {
        state: CommerceEntitlementState {
            state: "active".into(),
            ..Default::default()
        },
        valid_from_epoch_ms: Some(1),
        valid_until_epoch_ms: Some(10),
        ..Default::default()
    };
    assert!(grant.is_active_at(5));
}

#[test]
fn invalid_commerce_descriptor_is_rejected() {
    let mut invalid = commerce_catalog_pack_definition();
    invalid.pack_id = "pack.commerce.catalog.v2".into();
    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized commerce descriptor")
}

fn hash_values<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_value(value)
        .expect("descriptor hash DTO is serializable")
        .as_object()
        .expect("descriptor hash DTO serializes as an object")
        .values()
        .map(|value| {
            value
                .as_str()
                .expect("descriptor hash fields are strings")
                .to_string()
        })
        .collect()
}
