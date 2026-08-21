//! Manifest admission Specification for foreground/background lifecycle scopes.

use super::device_foreground_background_host::{
    DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID, HOST_LIFECYCLE_PERMISSION_SCOPES,
};
use super::model::AppServiceContractConfig;

/// Reject scopes outside the provider-neutral host-lifecycle descriptor.
pub fn validate_host_lifecycle_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get(DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID)
    else {
        return Ok(());
    };
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack| pack == DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID);
    if !declared {
        return Err("host lifecycle permissions require the foreground background host pack");
    }
    if scopes
        .iter()
        .any(|scope| !HOST_LIFECYCLE_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("host lifecycle permission scope is not declared by the pack");
    }
    Ok(())
}
