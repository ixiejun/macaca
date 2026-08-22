use crate::AppServiceContractConfig;
/// Validate market-data permissions against the descriptor-owned allowlist.
pub fn validate_finance_market_data_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "market_data.provider.inspect",
        "market_data.instrument.search",
        "market_data.instrument.read",
        "market_data.quote.read",
        "market_data.trade.read",
        "market_data.bars.read",
        "market_data.snapshot.read",
        "market_data.corporate_actions.read",
        "market_data.market_status.read",
        "market_data.freshness.read",
        "market_data.artifact.read",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.finance.market.data.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown market-data permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
