use crate::AppServiceContractConfig;

/// Validate payment-intent declarations against the provider-neutral permission taxonomy.
pub fn validate_commerce_payment_intent_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let allowed = [
        "commerce.payment.intent.read",
        "commerce.payment.intent.create",
        "commerce.payment.intent.confirm",
        "commerce.payment.intent.capture",
        "commerce.payment.intent.cancel",
        "commerce.payment.intent.audit_export",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.commerce.payment.intent.v1")
    {
        for scope in scopes {
            if !allowed.contains(&scope.as_str()) {
                return Err(format!("unknown payment intent permission scope: {scope}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_intent_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.commerce.payment.intent.v1".into(),
            ["commerce.payment.intent.read".into()]
                .into_iter()
                .collect(),
        );
        assert!(validate_commerce_payment_intent_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.commerce.payment.intent.v1")
            .unwrap()
            .insert("commerce.payment.intent.native".into());
        assert!(validate_commerce_payment_intent_permission_declarations(&declaration).is_err());
    }
}
