use macaca_proto::domain_pack_contract::{
    commerce_cart::{COMMERCE_CART_PACK_ID, COMMERCE_CART_SERVICE_ID},
    commerce_catalog::{COMMERCE_CATALOG_PACK_ID, COMMERCE_CATALOG_SERVICE_ID},
    commerce_entitlement::{COMMERCE_ENTITLEMENT_PACK_ID, COMMERCE_ENTITLEMENT_SERVICE_ID},
    commerce_order::{COMMERCE_ORDER_PACK_ID, COMMERCE_ORDER_SERVICE_ID},
    commerce_payment_intent::{
        COMMERCE_PAYMENT_INTENT_PACK_ID, COMMERCE_PAYMENT_INTENT_SERVICE_ID,
    },
    commerce_receipt::{COMMERCE_RECEIPT_PACK_ID, COMMERCE_RECEIPT_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// Commerce SDK tests validate catalog discovery only. The SDK must not create
// storefront, cart, order, payment, receipt, entitlement, checkout, or app
// feature-gating providers; it only reports descriptor metadata and unavailable
// diagnostics from the provider-neutral catalog.

#[tokio::test]
async fn catalog_client_discovers_commerce_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            COMMERCE_CATALOG_PACK_ID,
            COMMERCE_CATALOG_SERVICE_ID,
            "catalog.search_catalog",
            "commerce_catalog_provider_not_installed",
            "catalog-search",
        ),
        (
            COMMERCE_CART_PACK_ID,
            COMMERCE_CART_SERVICE_ID,
            "cart.plan_handoff",
            "commerce_cart_provider_not_installed",
            "cart-handoff",
        ),
        (
            COMMERCE_ORDER_PACK_ID,
            COMMERCE_ORDER_SERVICE_ID,
            "order.plan_state_transition",
            "commerce_order_provider_not_installed",
            "order-lifecycle",
        ),
        (
            COMMERCE_PAYMENT_INTENT_PACK_ID,
            COMMERCE_PAYMENT_INTENT_SERVICE_ID,
            "payment_intent.inspect_action",
            "commerce_payment_intent_provider_not_installed",
            "payment-action",
        ),
        (
            COMMERCE_RECEIPT_PACK_ID,
            COMMERCE_RECEIPT_SERVICE_ID,
            "receipt.delivery_request",
            "commerce_receipt_provider_not_installed",
            "receipt-delivery",
        ),
        (
            COMMERCE_ENTITLEMENT_PACK_ID,
            COMMERCE_ENTITLEMENT_SERVICE_ID,
            "entitlement.record_usage",
            "commerce_entitlement_provider_not_installed",
            "entitlement-usage",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid commerce id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("commerce descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/commerce"));
    }
}

#[tokio::test]
async fn catalog_client_reports_commerce_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                COMMERCE_CATALOG_PACK_ID.into(),
                COMMERCE_CART_PACK_ID.into(),
                COMMERCE_ORDER_PACK_ID.into(),
                COMMERCE_PAYMENT_INTENT_PACK_ID.into(),
                COMMERCE_RECEIPT_PACK_ID.into(),
                COMMERCE_ENTITLEMENT_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            COMMERCE_CATALOG_PACK_ID,
            "commerce_catalog_provider_not_installed",
        ),
        (
            COMMERCE_CART_PACK_ID,
            "commerce_cart_provider_not_installed",
        ),
        (
            COMMERCE_ORDER_PACK_ID,
            "commerce_order_provider_not_installed",
        ),
        (
            COMMERCE_PAYMENT_INTENT_PACK_ID,
            "commerce_payment_intent_provider_not_installed",
        ),
        (
            COMMERCE_RECEIPT_PACK_ID,
            "commerce_receipt_provider_not_installed",
        ),
        (
            COMMERCE_ENTITLEMENT_PACK_ID,
            "commerce_entitlement_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
