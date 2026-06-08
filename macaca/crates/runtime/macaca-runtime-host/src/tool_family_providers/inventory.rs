//! Governance inventory assembly for industrial tool families.
//!
//! Produces sanitized rows that mirror descriptor metadata without raw provider payloads.
//! Governance tests and diagnostics can inspect this structure without reaching into
//! provider-specific configuration.

use std::collections::BTreeMap;

use macaca_proto::MacacaResult;

use super::constants::TOOL_FAMILY_AUDIT_NAMESPACE;
use super::family_catalog::family_specs;

/// Inventory row used by governance notes, tests, and descriptor generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialToolFamilyProviderInventory {
    pub family: String,
    pub owner_service: String,
    pub provider_id: String,
    pub capability_id: String,
    pub tool_name: String,
    pub provider_path: String,
    pub sanitized_metadata: BTreeMap<String, String>,
}

/// Return the complete family inventory with only sanitized operational data.
pub fn industrial_tool_family_provider_inventory(
) -> MacacaResult<Vec<IndustrialToolFamilyProviderInventory>> {
    let specs = family_specs();
    tracing::info!(
        family_count = specs.len(),
        "industrial tool family provider inventory assembled"
    );
    specs
        .into_iter()
        .map(|spec| {
            // The inventory intentionally mirrors descriptor data without raw provider payloads.
            // Governance tests can inspect this structure without needing access to
            // provider-specific configuration.
            let mut sanitized_metadata = BTreeMap::new();
            sanitized_metadata.insert("provider_path".into(), spec.provider_path.to_string());
            sanitized_metadata.insert("owner_service".into(), spec.owner_service.clone());
            sanitized_metadata.insert("availability_state".into(), spec.availability_state());
            sanitized_metadata.insert("extension_point".into(), spec.extension_point.to_string());
            // Stable audit evidence for generic tool-family descriptors. Metadata only — not a
            // routing branch — so service.tool can replay catalog provenance safely.
            sanitized_metadata.insert(
                "service_namespace".into(),
                TOOL_FAMILY_AUDIT_NAMESPACE.into(),
            );
            Ok(IndustrialToolFamilyProviderInventory {
                family: spec.family.to_string(),
                owner_service: spec.owner_service,
                provider_id: spec.provider_id,
                capability_id: spec.capability_id,
                tool_name: spec.tool_name,
                provider_path: spec.provider_path.to_string(),
                sanitized_metadata,
            })
        })
        .collect()
}
