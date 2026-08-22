use super::identity_profile::IDENTITY_PROFILE_PERMISSION_SCOPES;
use crate::AppServiceContractConfig;

/// Validate profile permission declarations against descriptor-owned scopes.
pub fn validate_identity_profile_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.identity.profile.v1")
    else {
        return Ok(());
    };
    for scope in scopes {
        if !IDENTITY_PROFILE_PERMISSION_SCOPES.contains(&scope.as_str()) {
            return Err(format!(
                "unknown identity profile permission scope: {scope}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.identity.profile.v1".into(),
            ["identity.profile.read".into()].into_iter().collect(),
        );
        assert!(validate_identity_profile_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.identity.profile.v1")
            .unwrap()
            .insert("identity.profile.native".into());
        assert!(validate_identity_profile_permission_declarations(&declaration).is_err());
    }
}
