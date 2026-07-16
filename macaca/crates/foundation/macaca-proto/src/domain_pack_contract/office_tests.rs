use std::collections::{BTreeMap, BTreeSet};

use super::office_common::OfficeCommandEnvelope;
use super::office_document::*;
use super::office_forms::*;
use super::office_pdf::*;
use super::office_presentation::*;
use super::office_spreadsheet::*;
use super::*;

// Office pack tests validate provider-neutral contract shape only. They do not
// load document files, decrypt PDFs, publish forms, call cloud APIs, construct
// provider adapters, or expose raw office content in test fixtures.

#[test]
fn office_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            office_document_pack_definition(),
            OFFICE_DOCUMENT_PACK_ID,
            OFFICE_DOCUMENT_SERVICE_ID,
            OFFICE_DOCUMENT_COMMANDS,
            "office_document_provider_not_installed",
            "structured-document",
            "document.open_document",
        ),
        (
            office_spreadsheet_pack_definition(),
            OFFICE_SPREADSHEET_PACK_ID,
            OFFICE_SPREADSHEET_SERVICE_ID,
            OFFICE_SPREADSHEET_COMMANDS,
            "office_spreadsheet_provider_not_installed",
            "workbook-grid",
            "spreadsheet.open_workbook",
        ),
        (
            office_presentation_pack_definition(),
            OFFICE_PRESENTATION_PACK_ID,
            OFFICE_PRESENTATION_SERVICE_ID,
            OFFICE_PRESENTATION_COMMANDS,
            "office_presentation_provider_not_installed",
            "deck-structure",
            "presentation.open_deck",
        ),
        (
            office_pdf_pack_definition(),
            OFFICE_PDF_PACK_ID,
            OFFICE_PDF_SERVICE_ID,
            OFFICE_PDF_COMMANDS,
            "office_pdf_provider_not_installed",
            "pdf-structure",
            "pdf.open_document",
        ),
        (
            office_forms_pack_definition(),
            OFFICE_FORMS_PACK_ID,
            OFFICE_FORMS_SERVICE_ID,
            OFFICE_FORMS_COMMANDS,
            "office_forms_provider_not_installed",
            "form-schema",
            "forms.open_form",
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
            Some("pack.office.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/office"));
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
            .expect("office descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_office_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let document = definitions
        .iter()
        .find(|definition| definition.pack_id == OFFICE_DOCUMENT_PACK_ID)
        .expect("industrial catalog includes office document");
    let pdf = definitions
        .iter()
        .find(|definition| definition.pack_id == OFFICE_PDF_PACK_ID)
        .expect("industrial catalog includes office PDF");
    let forms = definitions
        .iter()
        .find(|definition| definition.pack_id == OFFICE_FORMS_PACK_ID)
        .expect("industrial catalog includes office forms");

    assert_eq!(
        document.metadata.diagnostics.unavailable_reason,
        "office_document_provider_not_installed"
    );
    assert!(document
        .metadata
        .service_command_schemas
        .get(OFFICE_DOCUMENT_SERVICE_ID)
        .is_some_and(|commands| commands.contains("document.open_document")));
    assert_eq!(
        pdf.metadata
            .provider_descriptors
            .get("pdf-security")
            .and_then(|descriptor| descriptor.metadata.get("signatures"))
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        forms
            .metadata
            .provider_descriptors
            .get("form-response")
            .and_then(|descriptor| descriptor.metadata.get("webhook"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn office_command_dtos_are_serde_compatible() {
    let envelope = OfficeCommandEnvelope {
        subject_ref: "office:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "preview".into())]),
        cursor: None,
        page_size: Some(25),
        idempotency_key: Some("idem-office".into()),
    };

    let values = [
        serde_json::to_value(DocumentOpenDocumentCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(SpreadsheetOpenWorkbookCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(PresentationOpenDeckCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(PdfOpenDocumentCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(FormsOpenFormCommand { request: envelope }).unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn office_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        office_document_descriptor_hashes().into_hashes(),
        office_spreadsheet_descriptor_hashes().into_hashes(),
        office_presentation_descriptor_hashes().into_hashes(),
        office_pdf_descriptor_hashes().into_hashes(),
        office_forms_descriptor_hashes().into_hashes(),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), 6);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn office_validation_helpers_are_provider_neutral() {
    let range = DocumentRange {
        range_id: "range".into(),
        anchor_hash: "anchor".into(),
        start_offset: 10,
        end_offset: 20,
    };
    assert!(range.is_bounded(64));

    let grid = SpreadsheetRange {
        worksheet_id: "sheet".into(),
        anchor_hash: "anchor".into(),
        row_start: 1,
        row_end: 10,
        column_start: 1,
        column_end: 5,
    };
    assert_eq!(grid.cell_count(), 50);

    let page = PdfPageHandle {
        document_id: "pdf".into(),
        page_index: 2,
        page_anchor_hash: "page-anchor".into(),
    };
    assert!(page.is_inside_page_count(3));
    let render = PdfRenderPlan {
        page,
        width_px: 800,
        height_px: 600,
        redaction_profile: "preview".into(),
    };
    assert_eq!(render.pixel_budget(), 480_000);

    let schema = FormSchema {
        schema_id: "schema".into(),
        version_hash: "v1".into(),
        sections: vec![FormSection {
            section_id: "section".into(),
            title_ref: None,
            fields: vec![FormField::default(), FormField::default()],
        }],
    };
    assert_eq!(schema.field_count(), 2);
}

#[test]
fn invalid_office_descriptor_is_rejected() {
    let mut invalid = office_document_pack_definition();
    invalid.pack_id = "pack.office.document.v2".into();
    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

trait DescriptorHashSet {
    fn into_hashes(self) -> [String; 6];
}

impl DescriptorHashSet for DocumentDescriptorHashes {
    fn into_hashes(self) -> [String; 6] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.document_version_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for SpreadsheetDescriptorHashes {
    fn into_hashes(self) -> [String; 6] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.workbook_version_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for PresentationDescriptorHashes {
    fn into_hashes(self) -> [String; 6] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.deck_version_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for PdfDescriptorHashes {
    fn into_hashes(self) -> [String; 6] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.document_version_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for FormsDescriptorHashes {
    fn into_hashes(self) -> [String; 6] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.form_version_hash,
            self.unavailable_schema_hash,
        ]
    }
}
