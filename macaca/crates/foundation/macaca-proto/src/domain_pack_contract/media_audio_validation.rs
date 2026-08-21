//! Admission Specification for media-audio permission declarations.
//!
//! The protocol validates only descriptor-owned scope names. Runtime policy,
//! entitlement, consent, resource, and approval checks remain outside manifests.

use super::media_audio::{AUDIO_PERMISSION_SCOPES, MEDIA_AUDIO_PACK_ID};
use super::model::AppServiceContractConfig;

/// Reject unknown audio scopes before the ABI projects callable imports.
pub fn validate_audio_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration.pack_permission_scopes.get(MEDIA_AUDIO_PACK_ID) else {
        return Ok(());
    };
    if !declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack_id| pack_id == MEDIA_AUDIO_PACK_ID)
    {
        return Err("audio permissions require the media audio pack");
    }
    if scopes
        .iter()
        .any(|scope| !AUDIO_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("audio permission scope is not declared by the pack");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn audio_permissions_are_pack_scoped_and_descriptor_owned() {
        let valid = AppServiceContractConfig {
            optional_packs: vec![MEDIA_AUDIO_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                MEDIA_AUDIO_PACK_ID.into(),
                BTreeSet::from(["audio.export".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_audio_permission_declarations(&valid).is_ok());

        let unknown = AppServiceContractConfig {
            optional_packs: vec![MEDIA_AUDIO_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                MEDIA_AUDIO_PACK_ID.into(),
                BTreeSet::from(["audio.provider.native".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_audio_permission_declarations(&unknown).is_err());
    }
}
