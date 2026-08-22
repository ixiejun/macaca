use crate::AppServiceContractConfig;

/// Validate order permission declarations against the descriptor-owned allowlist.
pub fn validate_commerce_order_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let allowed = [
        "commerce.order.read",
        "commerce.order.write",
        "commerce.order.status",
        "commerce.order.fulfillment_intent",
        "commerce.order.cancel",
        "commerce.order.audit_export",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.commerce.order.v1")
    {
        for scope in scopes {
            if !allowed.contains(&scope.as_str()) {
                return Err(format!("unknown commerce order permission scope: {scope}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.commerce.order.v1".into(),
            ["commerce.order.read".into()].into_iter().collect(),
        );
        assert!(validate_commerce_order_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.commerce.order.v1")
            .unwrap()
            .insert("commerce.order.native".into());
        assert!(validate_commerce_order_permission_declarations(&declaration).is_err());
    }
}
