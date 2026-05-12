//! WASM Component Application ABI schema helpers.
//!
//! This module is deliberately data-only.  It exposes the checked-in WIT
//! schema as a static string and provides tiny parser helpers for alignment
//! tests.  Keeping this logic in `macaca-proto` lets SDKs, package validators,
//! and future hosts agree on names without importing `macaca-app`,
//! `macaca-runtime-host`, Web state, or any concrete WASM execution engine.

use crate::{ApplicationExport, ApplicationImport};

/// Canonical WIT schema for Macaca Application Component ABI v0.
///
/// The file is included at compile time so drift tests validate the exact
/// schema shipped with the repository.  The schema remains a skeleton: it
/// declares the future guest/host boundary but does not imply that this build
/// contains a WASM runtime.
pub const MACACA_APPLICATION_WIT: &str =
    include_str!("../../../../application-wit/macaca-application.wit");

const IMPORT_MARKER: &str = "canonical-import:";
const EXPORT_MARKER: &str = "canonical-export:";

/// Return canonical import labels declared by the checked-in WIT schema.
///
/// The parser intentionally understands only the comment markers used by our
/// schema.  It is not a general WIT parser; this keeps the ABI drift check
/// lightweight and avoids adding a parser dependency before a real WASM
/// runtime proposal is approved.
pub fn wit_canonical_import_names() -> Vec<String> {
    marker_values(MACACA_APPLICATION_WIT, IMPORT_MARKER)
}

/// Return canonical export labels declared by the checked-in WIT schema.
///
/// Export labels are compared with `ApplicationExport::required_v0()` in tests
/// so the WIT skeleton cannot silently drift from the Rust protocol DTOs.
pub fn wit_canonical_export_names() -> Vec<String> {
    marker_values(MACACA_APPLICATION_WIT, EXPORT_MARKER)
}

/// Return canonical Rust import labels for Application ABI v0.
pub fn rust_application_import_names_v0() -> Vec<String> {
    ApplicationImport::v0()
        .into_iter()
        .map(|import| import.as_name().to_string())
        .collect()
}

/// Return canonical Rust export labels for Application ABI v0.
pub fn rust_application_export_names_v0() -> Vec<String> {
    ApplicationExport::required_v0()
        .into_iter()
        .map(|export| export.as_name().to_string())
        .collect()
}

fn marker_values(schema: &str, marker: &str) -> Vec<String> {
    schema
        .lines()
        .filter_map(|line| line.split_once(marker).map(|(_, value)| value.trim()))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_abi_wit_imports_match_rust_dtos() {
        assert_eq!(
            wit_canonical_import_names(),
            rust_application_import_names_v0()
        );
    }

    #[test]
    fn application_abi_wit_exports_match_rust_dtos() {
        assert_eq!(
            wit_canonical_export_names(),
            rust_application_export_names_v0()
        );
    }

    #[test]
    fn application_abi_wit_is_runtime_skeleton_only() {
        assert!(MACACA_APPLICATION_WIT.contains("world application-v0"));
        assert!(MACACA_APPLICATION_WIT.contains("does not execute guest WASM bytes"));
    }
}
