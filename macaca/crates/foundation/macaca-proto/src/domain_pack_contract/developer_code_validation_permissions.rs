use crate::AppServiceContractConfig;

/// Validate code-pack permissions using the descriptor-owned scope allowlist.
pub fn validate_developer_code_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "code.workspace.read",
        "code.workspace.index",
        "code.document.read",
        "code.document.parse",
        "code.symbol.read",
        "code.diagnostic.read",
        "code.action.read",
        "code.edit.plan",
        "code.patch.generate",
        "code.patch.validate",
        "code.patch.apply",
        "code.diff.read",
        "code.impact.read",
        "code.test.suggest",
        "code.scan.import",
        "code.scan.read",
        "code.provider.inspect",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.code.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown code permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
