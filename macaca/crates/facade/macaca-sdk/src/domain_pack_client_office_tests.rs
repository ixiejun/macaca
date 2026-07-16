use macaca_proto::domain_pack_contract::{
    office_document::{OFFICE_DOCUMENT_PACK_ID, OFFICE_DOCUMENT_SERVICE_ID},
    office_forms::{OFFICE_FORMS_PACK_ID, OFFICE_FORMS_SERVICE_ID},
    office_pdf::{OFFICE_PDF_PACK_ID, OFFICE_PDF_SERVICE_ID},
    office_presentation::{OFFICE_PRESENTATION_PACK_ID, OFFICE_PRESENTATION_SERVICE_ID},
    office_spreadsheet::{OFFICE_SPREADSHEET_PACK_ID, OFFICE_SPREADSHEET_SERVICE_ID},
};
use macaca_proto::{compose_installed_domain_pack_catalog, reference_domain_pack_definitions};

use super::*;

// These tests keep the Office SDK path provider-neutral. The SDK reads catalog
// metadata and never constructs document, spreadsheet, presentation, PDF, forms,
// cloud-suite, renderer, signer, webhook, or conversion providers.

#[tokio::test]
async fn catalog_client_discovers_office_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            OFFICE_DOCUMENT_PACK_ID,
            OFFICE_DOCUMENT_SERVICE_ID,
            "document.open_document",
            "office_document_provider_not_installed",
            "structured-document",
        ),
        (
            OFFICE_SPREADSHEET_PACK_ID,
            OFFICE_SPREADSHEET_SERVICE_ID,
            "spreadsheet.open_workbook",
            "office_spreadsheet_provider_not_installed",
            "workbook-grid",
        ),
        (
            OFFICE_PRESENTATION_PACK_ID,
            OFFICE_PRESENTATION_SERVICE_ID,
            "presentation.open_deck",
            "office_presentation_provider_not_installed",
            "deck-structure",
        ),
        (
            OFFICE_PDF_PACK_ID,
            OFFICE_PDF_SERVICE_ID,
            "pdf.open_document",
            "office_pdf_provider_not_installed",
            "pdf-structure",
        ),
        (
            OFFICE_FORMS_PACK_ID,
            OFFICE_FORMS_SERVICE_ID,
            "forms.open_form",
            "office_forms_provider_not_installed",
            "form-schema",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid office id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("office descriptor exists");

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
            .contains("developer-packs/office"));
    }
}
