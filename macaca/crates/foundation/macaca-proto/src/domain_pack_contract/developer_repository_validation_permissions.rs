use crate::AppServiceContractConfig;

/// Validate repository permissions before an application can call the service boundary.
pub fn validate_developer_repository_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "repository.local.read",
        "repository.local.write",
        "repository.status.read",
        "repository.diff.read",
        "repository.history.read",
        "repository.ref.read",
        "repository.ref.write",
        "repository.stage.write",
        "repository.commit.create",
        "repository.remote.read",
        "repository.remote.fetch",
        "repository.remote.push",
        "repository.remote.metadata",
        "repository.mutation.plan",
        "repository.mutation.validate",
        "repository.provider.inspect",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.repository.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown repository permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
