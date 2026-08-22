use crate::AppServiceContractConfig;
/// Validate crypto read permissions against the descriptor-owned allowlist.
pub fn validate_finance_crypto_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "crypto.provider.inspect",
        "crypto.asset.search",
        "crypto.asset.read",
        "crypto.token.read",
        "crypto.market_pair.search",
        "crypto.quote.read",
        "crypto.trade.read",
        "crypto.bars.read",
        "crypto.snapshot.read",
        "crypto.supply.read",
        "crypto.market_status.read",
        "crypto.public_address.read",
        "crypto.freshness.read",
        "crypto.artifact.read",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.finance.crypto.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown crypto permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
