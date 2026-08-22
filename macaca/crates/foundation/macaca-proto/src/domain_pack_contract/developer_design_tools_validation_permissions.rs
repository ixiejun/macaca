use crate::AppServiceContractConfig;

/// Validate design-tool permission scopes against the descriptor-owned allowlist.
pub fn validate_developer_design_tools_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "design_tools.provider.inspect",
        "design_tools.workspace.read",
        "design_tools.file.read",
        "design_tools.page.read",
        "design_tools.node.read",
        "design_tools.component.read",
        "design_tools.token.read",
        "design_tools.token.write",
        "design_tools.asset.export",
        "design_tools.component.map",
        "design_tools.design.write",
        "design_tools.review.read",
        "design_tools.artifact.read",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.design.tools.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown design-tools permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
