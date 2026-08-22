use crate::AppServiceContractConfig;
/// Validate terminal permissions against the descriptor-owned allowlist.
pub fn validate_developer_terminal_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "terminal.provider.inspect",
        "terminal.spawn",
        "terminal.stream.read",
        "terminal.stdin.write",
        "terminal.resize",
        "terminal.process.inspect",
        "terminal.exit.collect",
        "terminal.cancel",
        "terminal.workdir.snapshot",
        "terminal.session.cleanup",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.terminal.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown terminal permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
