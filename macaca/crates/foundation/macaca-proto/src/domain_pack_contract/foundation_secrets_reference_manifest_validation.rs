//! Manifest admission Specification for provider-neutral secret references.

use std::collections::BTreeSet;

use super::foundation_secrets_reference::{SecretReference, FOUNDATION_SECRETS_REFERENCE_PACK_ID};
use super::model::AppServiceContractConfig;

/// Validate reference-only declarations before catalog expansion or provider selection.
pub fn validate_secret_reference_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack_id| pack_id == FOUNDATION_SECRETS_REFERENCE_PACK_ID);
    if !declaration.secret_reference_declarations.is_empty() && !declared {
        return Err("secret references require the foundation secrets-reference pack");
    }
    let mut references = BTreeSet::new();
    for reference in &declaration.secret_reference_declarations {
        if !reference.is_safe_reference() {
            return Err("secret reference metadata is unsafe or unbounded");
        }
        if !references.insert(&reference.reference_id) {
            return Err("secret reference ids must be unique");
        }
    }
    Ok(())
}

impl SecretReference {
    /// Keep manifest declarations as opaque metadata; raw values and locators are not representable.
    pub fn is_manifest_declaration(&self) -> bool {
        self.is_safe_reference()
    }
}
