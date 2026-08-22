//! Manifest admission Specification for local-files permission scopes.

use super::device_local_files::{DEVICE_LOCAL_FILES_PACK_ID, LOCAL_FILES_PERMISSION_SCOPES};
use super::model::AppServiceContractConfig;

/// Reject local-files permission declarations outside the descriptor vocabulary.
pub fn validate_local_files_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get(DEVICE_LOCAL_FILES_PACK_ID)
    else {
        return Ok(());
    };
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack| pack == DEVICE_LOCAL_FILES_PACK_ID);
    if !declared {
        return Err("local files permissions require the local files pack");
    }
    if scopes
        .iter()
        .any(|scope| !LOCAL_FILES_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("local files permission scope is not declared by the pack");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn local_files_permissions_are_descriptor_owned() {
        let valid = AppServiceContractConfig {
            optional_packs: vec![DEVICE_LOCAL_FILES_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                DEVICE_LOCAL_FILES_PACK_ID.into(),
                BTreeSet::from(["device.local_files.read".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_local_files_permission_declarations(&valid).is_ok());
        let invalid = AppServiceContractConfig {
            optional_packs: vec![DEVICE_LOCAL_FILES_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                DEVICE_LOCAL_FILES_PACK_ID.into(),
                BTreeSet::from(["device.local_files.native".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_local_files_permission_declarations(&invalid).is_err());
    }
}
