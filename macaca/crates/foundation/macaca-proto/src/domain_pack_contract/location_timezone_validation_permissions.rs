use super::location_timezone::TIMEZONE_PERMISSION_SCOPES;
use crate::AppServiceContractConfig;

/// Validate timezone permission declarations against descriptor-owned scopes.
pub fn validate_location_timezone_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.location.timezone.v1")
    else {
        return Ok(());
    };
    for scope in scopes {
        if !TIMEZONE_PERMISSION_SCOPES.contains(&scope.as_str()) {
            return Err(format!(
                "unknown location timezone permission scope: {scope}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timezone_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.location.timezone.v1".into(),
            ["location.timezone.lookup.read".into()]
                .into_iter()
                .collect(),
        );
        assert!(validate_location_timezone_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.location.timezone.v1")
            .unwrap()
            .insert("location.timezone.native".into());
        assert!(validate_location_timezone_permission_declarations(&declaration).is_err());
    }
}
