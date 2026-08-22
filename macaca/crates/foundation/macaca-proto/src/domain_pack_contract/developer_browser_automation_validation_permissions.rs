use crate::AppServiceContractConfig;

/// Validate browser automation permissions without binding an engine or driver.
pub fn validate_developer_browser_automation_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    let allowed = [
        "browser.provider.inspect",
        "browser.context.open",
        "browser.context.close",
        "browser.page.open",
        "browser.page.close",
        "browser.navigate",
        "browser.wait",
        "browser.dom.inspect",
        "browser.locator.resolve",
        "browser.action.perform",
        "browser.evaluate",
        "browser.screenshot",
        "browser.accessibility.inspect",
        "browser.download.manage",
        "browser.upload.manage",
        "browser.events.inspect",
        "browser.storage.manage",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.browser.automation.v1")
    {
        for scope in scopes {
            if !allowed.contains(&scope.as_str()) {
                return Err(format!("unknown browser permission scope: {scope}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_permissions_reject_provider_native_scope() {
        let mut declaration = AppServiceContractConfig::default();
        declaration.pack_permission_scopes.insert(
            "pack.developer.browser.automation.v1".into(),
            ["browser.native".into()].into_iter().collect(),
        );
        assert!(
            validate_developer_browser_automation_permission_declarations(&declaration).is_err()
        );
    }
}
