use super::model::{AppServiceContractConfig, DomainPackDefinition, DomainPackMetadata};
use crate::{ServiceError, ServiceResult};

/// Specification for application-side pack declarations.
///
/// Admission can validate manifest syntax without loading provider crates.  Runtime capability
/// expansion still owns availability checks through the catalog Strategy.
#[derive(Debug, Clone, Copy, Default)]
pub struct AppServiceContractSpec;

impl AppServiceContractSpec {
    pub fn validate(&self, contract: &AppServiceContractConfig) -> ServiceResult<()> {
        for pack_id in contract
            .use_packs
            .iter()
            .chain(contract.required_packs.iter())
            .chain(contract.optional_packs.iter())
            .chain(contract.pack_policy_overrides.keys())
        {
            validate_domain_pack_id(pack_id)?;
        }
        Ok(())
    }
}

/// Specification for domain-pack identifiers and version strings.
#[derive(Debug, Clone, Copy, Default)]
pub struct DomainPackIdentitySpec;

impl DomainPackIdentitySpec {
    pub fn validate(&self, definition: &DomainPackDefinition) -> ServiceResult<()> {
        validate_domain_pack_id(&definition.pack_id)?;
        validate_pack_metadata(&definition.pack_id, &definition.metadata)?;
        Ok(())
    }
}

/// Specification for parent/child pack hierarchy compatibility.
#[derive(Debug, Clone, Copy, Default)]
pub struct DomainPackHierarchySpec;

impl DomainPackHierarchySpec {
    pub fn validate_parent_child(
        &self,
        parent_pack_id: &str,
        child_pack_id: &str,
    ) -> ServiceResult<()> {
        validate_domain_pack_parent(parent_pack_id, child_pack_id)
    }
}

/// Specification for full pack definition validation.
#[derive(Debug, Clone, Copy, Default)]
pub struct DomainPackDefinitionSpec;

impl DomainPackDefinitionSpec {
    pub fn validate(&self, definition: &DomainPackDefinition) -> ServiceResult<()> {
        DomainPackIdentitySpec.validate(definition)?;
        if let Some(parent_pack_id) = definition.metadata.parent_pack_id.as_deref() {
            DomainPackHierarchySpec.validate_parent_child(parent_pack_id, &definition.pack_id)?;
        }
        Ok(())
    }
}

/// Validate a public pack id such as `pack.finance.stock.v1`.
pub fn validate_domain_pack_id(pack_id: &str) -> ServiceResult<()> {
    let pack_id = pack_id.trim();
    if !pack_id.starts_with("pack.") {
        return Err(ServiceError::InvalidArgument(
            "domain pack id must start with `pack.`".into(),
        ));
    }
    let Some((family_path, version)) = pack_id.rsplit_once(".v") else {
        return Err(ServiceError::InvalidArgument(
            "domain pack id must end with `.vN`".into(),
        ));
    };
    validate_domain_pack_version(&format!("v{version}"))?;
    for segment in family_path.trim_start_matches("pack.").split('.') {
        validate_domain_pack_family_id(segment)?;
    }
    Ok(())
}

/// Validate a taxonomy segment without assuming any concrete business family.
pub fn validate_domain_pack_family_id(family_id: &str) -> ServiceResult<()> {
    if family_id.is_empty() || !family_id.chars().all(is_pack_segment_char) {
        return Err(ServiceError::InvalidArgument(
            "domain pack family segments must be non-empty and alphanumeric".into(),
        ));
    }
    Ok(())
}

/// Validate the stable `vN` version shape used by pack ids.
pub fn validate_domain_pack_version(version: &str) -> ServiceResult<()> {
    let Some(number) = version.strip_prefix('v') else {
        return Err(ServiceError::InvalidArgument(
            "domain pack version must start with `v`".into(),
        ));
    };
    if number.is_empty() || !number.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(ServiceError::InvalidArgument(
            "domain pack version suffix must be numeric".into(),
        ));
    }
    Ok(())
}

/// Validate that a child pack extends its parent family and shares the same major version.
pub fn validate_domain_pack_parent(parent_pack_id: &str, child_pack_id: &str) -> ServiceResult<()> {
    validate_domain_pack_id(parent_pack_id)?;
    validate_domain_pack_id(child_pack_id)?;
    let Some((parent_prefix, parent_version)) = parent_pack_id.rsplit_once(".v") else {
        return Err(ServiceError::InvalidArgument(
            "parent pack id must end with `.vN`".into(),
        ));
    };
    let Some((child_prefix, child_version)) = child_pack_id.rsplit_once(".v") else {
        return Err(ServiceError::InvalidArgument(
            "child pack id must end with `.vN`".into(),
        ));
    };
    if parent_version != child_version {
        return Err(ServiceError::InvalidArgument(
            "parent and child pack versions must match".into(),
        ));
    }
    if !child_prefix.starts_with(parent_prefix) || child_prefix == parent_prefix {
        return Err(ServiceError::InvalidArgument(
            "child pack id must extend the parent pack family".into(),
        ));
    }
    Ok(())
}

fn validate_pack_metadata(pack_id: &str, metadata: &DomainPackMetadata) -> ServiceResult<()> {
    validate_domain_pack_family_id(metadata.family_id.trim())?;
    validate_domain_pack_version(metadata.version.trim())?;
    for version_range in [
        metadata.compatibility.version_range.as_str(),
        metadata.compatibility.parent_version_range.as_str(),
    ] {
        if !version_range.trim().is_empty() && version_range.trim().len() < 2 {
            return Err(ServiceError::InvalidArgument(format!(
                "domain pack `{pack_id}` uses an invalid compatibility range"
            )));
        }
    }
    if let Some(parent_pack_id) = metadata.parent_pack_id.as_deref() {
        validate_domain_pack_id(parent_pack_id)?;
    }
    Ok(())
}

fn is_pack_segment_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}
