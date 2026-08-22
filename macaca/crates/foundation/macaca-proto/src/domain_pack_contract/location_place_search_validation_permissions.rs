use super::location_place_search::PLACE_SEARCH_PERMISSION_SCOPES;
use crate::AppServiceContractConfig;

/// Validate place-search permission declarations against descriptor-owned scopes.
pub fn validate_location_place_search_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.location.place.search.v1")
    else {
        return Ok(());
    };
    for scope in scopes {
        if !PLACE_SEARCH_PERMISSION_SCOPES.contains(&scope.as_str()) {
            return Err(format!(
                "unknown location place-search permission scope: {scope}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_search_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.location.place.search.v1".into(),
            ["location.place.search.read".into()].into_iter().collect(),
        );
        assert!(validate_location_place_search_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.location.place.search.v1")
            .unwrap()
            .insert("location.place.native".into());
        assert!(validate_location_place_search_permission_declarations(&declaration).is_err());
    }
}
