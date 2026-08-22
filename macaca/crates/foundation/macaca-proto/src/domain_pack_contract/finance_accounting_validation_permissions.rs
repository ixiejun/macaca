use super::finance_accounting::ACCOUNTING_PERMISSION_SCOPES;
use crate::AppServiceContractConfig;

/// Validate accounting permission declarations against descriptor-owned scopes.
pub fn validate_finance_accounting_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.finance.accounting.v1")
    else {
        return Ok(());
    };
    for scope in scopes {
        if !ACCOUNTING_PERMISSION_SCOPES.contains(&scope.as_str()) {
            return Err(format!(
                "unknown finance accounting permission scope: {scope}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounting_permissions_are_descriptor_owned() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.finance.accounting.v1".into(),
            ["finance.accounting.read".into()].into_iter().collect(),
        );
        assert!(validate_finance_accounting_permission_declarations(&declaration).is_ok());
        declaration
            .pack_permission_scopes
            .get_mut("pack.finance.accounting.v1")
            .unwrap()
            .insert("finance.accounting.native".into());
        assert!(validate_finance_accounting_permission_declarations(&declaration).is_err());
    }
}
