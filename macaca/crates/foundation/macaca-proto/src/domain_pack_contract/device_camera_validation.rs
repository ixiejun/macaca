//! Manifest admission Specification for descriptor-owned camera permissions.
//!
//! This validates permission names only. Authorization, foreground, privacy,
//! resource, and approval decisions remain runtime-host responsibilities.

use super::device_camera::{CAMERA_PERMISSION_SCOPES, DEVICE_CAMERA_PACK_ID};
use super::model::AppServiceContractConfig;

/// Reject camera permission declarations outside the pack descriptor vocabulary.
pub fn validate_camera_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get(DEVICE_CAMERA_PACK_ID)
    else {
        return Ok(());
    };
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack| pack == DEVICE_CAMERA_PACK_ID);
    if !declared {
        return Err("camera permissions require the device camera pack");
    }
    if scopes
        .iter()
        .any(|scope| !CAMERA_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("camera permission scope is not declared by the pack");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    #[test]
    fn camera_permissions_are_descriptor_owned() {
        let declaration = AppServiceContractConfig {
            optional_packs: vec![DEVICE_CAMERA_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                DEVICE_CAMERA_PACK_ID.into(),
                BTreeSet::from(["device.camera.capture_photo".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_camera_permission_declarations(&declaration).is_ok());
        let unknown = AppServiceContractConfig {
            optional_packs: vec![DEVICE_CAMERA_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                DEVICE_CAMERA_PACK_ID.into(),
                BTreeSet::from(["device.camera.native".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_camera_permission_declarations(&unknown).is_err());
    }
}
