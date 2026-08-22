use super::identity_account::IDENTITY_ACCOUNT_PERMISSION_SCOPES;
use crate::AppServiceContractConfig;

/// Validate account permission declarations against descriptor-owned scopes.
pub fn validate_identity_account_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.identity.account.v1")
    else {
        return Ok(());
    };
    for scope in scopes {
        if !IDENTITY_ACCOUNT_PERMISSION_SCOPES.contains(&scope.as_str()) {
            return Err(format!(
                "unknown identity account permission scope: {scope}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.identity.account.v1".into(),
            ["identity.account.read".into()].into_iter().collect(),
        );
        assert!(validate_identity_account_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.identity.account.v1")
            .unwrap()
            .insert("identity.account.native".into());
        assert!(validate_identity_account_permission_declarations(&declaration).is_err());
    }
}
