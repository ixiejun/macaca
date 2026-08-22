use super::commerce_catalog::CatalogScope;
use crate::AppServiceContractConfig;

/// Validate commerce catalog permission declarations against the provider-neutral scope set.
pub fn validate_commerce_catalog_permission_declarations(
    contract: &AppServiceContractConfig,
) -> Result<(), String> {
    let allowed = [
        "commerce.catalog.read",
        "commerce.catalog.search",
        "commerce.catalog.price",
        "commerce.catalog.availability",
        "commerce.catalog.write",
        "commerce.catalog.publish",
        "commerce.catalog.export",
    ];
    if let Some(scopes) = contract
        .pack_permission_scopes
        .get("pack.commerce.catalog.v1")
    {
        for declaration in scopes {
            if !allowed.contains(&declaration.as_str()) {
                return Err(format!(
                    "unsupported commerce catalog permission: {declaration}"
                ));
            }
        }
    }
    Ok(())
}

impl CatalogScope {
    /// Scope references are bounded and opaque; provider credentials never cross this boundary.
    pub fn is_bounded(&self) -> bool {
        [
            &self.tenant_scope,
            &self.store_ref,
            &self.channel_ref,
            &self.locale,
            &self.currency,
        ]
        .iter()
        .all(|value| !value.is_empty() && value.len() <= 160 && !value.contains("://"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_catalog_permission() {
        let mut contract = AppServiceContractConfig::default();
        contract.pack_permission_scopes.insert(
            "pack.commerce.catalog.v1".into(),
            ["commerce.catalog.secret".into()].into_iter().collect(),
        );
        assert!(validate_commerce_catalog_permission_declarations(&contract).is_err());
    }
}
