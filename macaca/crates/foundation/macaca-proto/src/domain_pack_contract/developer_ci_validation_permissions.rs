use crate::AppServiceContractConfig;

/// Validate CI permission scopes using the descriptor-owned allowlist.
pub fn validate_developer_ci_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "ci.provider.inspect",
        "ci.project.read",
        "ci.pipeline.read",
        "ci.run.read",
        "ci.status.read",
        "ci.trigger.plan",
        "ci.trigger.request",
        "ci.cancel.plan",
        "ci.cancel.request",
        "ci.rerun.plan",
        "ci.rerun.request",
        "ci.log.read",
        "ci.artifact.read",
        "ci.test.read",
        "ci.environment.read",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.ci.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown CI permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
