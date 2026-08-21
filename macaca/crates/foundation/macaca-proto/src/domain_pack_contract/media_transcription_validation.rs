//! Admission Specification for media-transcription permission declarations.
//!
//! This contract validates names only. Policy, consent, entitlement, resource,
//! and provider capability decisions remain runtime-owned and execute before a
//! provider receives any audio, transcript, or artifact handle.

use super::media_transcription::{MEDIA_TRANSCRIPTION_PACK_ID, TRANSCRIPTION_PERMISSION_SCOPES};
use super::model::AppServiceContractConfig;

/// Reject unknown transcription scopes before ABI projection can expose imports.
pub fn validate_transcription_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get(MEDIA_TRANSCRIPTION_PACK_ID)
    else {
        return Ok(());
    };
    if !declares_transcription_pack(declaration) {
        return Err("transcription permissions require the media transcription pack");
    }
    if scopes
        .iter()
        .any(|scope| !TRANSCRIPTION_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("transcription permission scope is not declared by the pack");
    }
    Ok(())
}

fn declares_transcription_pack(declaration: &AppServiceContractConfig) -> bool {
    declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack_id| pack_id == MEDIA_TRANSCRIPTION_PACK_ID)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn transcription_permissions_are_pack_scoped_and_descriptor_owned() {
        let valid = AppServiceContractConfig {
            optional_packs: vec![MEDIA_TRANSCRIPTION_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                MEDIA_TRANSCRIPTION_PACK_ID.into(),
                BTreeSet::from(["transcription.stream".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_transcription_permission_declarations(&valid).is_ok());

        let unknown = AppServiceContractConfig {
            optional_packs: vec![MEDIA_TRANSCRIPTION_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                MEDIA_TRANSCRIPTION_PACK_ID.into(),
                BTreeSet::from(["transcription.provider.native".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_transcription_permission_declarations(&unknown).is_err());
    }
}
