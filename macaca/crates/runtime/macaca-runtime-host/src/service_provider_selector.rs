//! Provider selection strategy for generic service routing.
//!
//! This module is intentionally data-driven and application-agnostic. It never
//! hardcodes workflow names, application IDs, or business-domain constants.
//! The router can use this selector to choose a provider target deterministically
//! from request metadata and normalized health/cost/latency snapshots.

use std::collections::BTreeMap;

/// Strategy label used by callers to influence provider selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSelectionStrategy {
    /// Choose the lowest observed latency first.
    LatencyFirst,
    /// Choose the lowest normalized unit cost first.
    CostFirst,
    /// Choose the highest trust score first.
    TrustFirst,
    /// Keep using the previous provider in-session when possible.
    Sticky,
}

impl ProviderSelectionStrategy {
    /// Parse strategy from request metadata. Unknown values safely fall back to
    /// latency-first to keep behavior deterministic.
    pub fn from_metadata(value: Option<&str>) -> Self {
        match value.unwrap_or_default() {
            "latency_first" => Self::LatencyFirst,
            "cost_first" => Self::CostFirst,
            "trust_first" => Self::TrustFirst,
            "sticky" => Self::Sticky,
            _ => Self::LatencyFirst,
        }
    }
}

/// Normalized provider telemetry sample consumed by the selector.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderSnapshot {
    pub provider_id: String,
    pub healthy: bool,
    pub latency_ms: u64,
    pub unit_cost: u64,
    pub trust_score: u64,
}

/// Stateless selector with deterministic tie-breaking by provider id.
pub struct ProviderSelector;

impl ProviderSelector {
    /// Select the best provider using the requested strategy.
    ///
    /// The selector always filters unhealthy providers first. If all providers
    /// are unhealthy, it returns `None` so the caller can fail closed.
    pub fn select(
        strategy: ProviderSelectionStrategy,
        previous_provider: Option<&str>,
        snapshots: &[ProviderSnapshot],
    ) -> Option<String> {
        let mut candidates: Vec<&ProviderSnapshot> = snapshots
            .iter()
            .filter(|snapshot| snapshot.healthy)
            .collect();
        if candidates.is_empty() {
            return None;
        }
        match strategy {
            ProviderSelectionStrategy::Sticky => {
                if let Some(previous) = previous_provider {
                    if candidates
                        .iter()
                        .any(|snapshot| snapshot.provider_id == previous)
                    {
                        return Some(previous.to_string());
                    }
                }
                candidates.sort_by_key(|snapshot| (snapshot.latency_ms, &snapshot.provider_id));
                candidates
                    .first()
                    .map(|snapshot| snapshot.provider_id.clone())
            }
            ProviderSelectionStrategy::LatencyFirst => {
                candidates.sort_by_key(|snapshot| (snapshot.latency_ms, &snapshot.provider_id));
                candidates
                    .first()
                    .map(|snapshot| snapshot.provider_id.clone())
            }
            ProviderSelectionStrategy::CostFirst => {
                candidates.sort_by_key(|snapshot| (snapshot.unit_cost, &snapshot.provider_id));
                candidates
                    .first()
                    .map(|snapshot| snapshot.provider_id.clone())
            }
            ProviderSelectionStrategy::TrustFirst => {
                candidates.sort_by_key(|snapshot| {
                    (
                        std::cmp::Reverse(snapshot.trust_score),
                        &snapshot.provider_id,
                    )
                });
                candidates
                    .first()
                    .map(|snapshot| snapshot.provider_id.clone())
            }
        }
    }

    /// Decode provider snapshots from metadata keys.
    ///
    /// This helper supports host-side experiments without coupling to a single
    /// registry backend. Metadata format:
    /// - `providers`: comma-separated provider ids.
    /// - `provider.<id>.healthy`: `true` or `false`.
    /// - `provider.<id>.latency_ms`: u64.
    /// - `provider.<id>.unit_cost`: u64.
    /// - `provider.<id>.trust_score`: u64.
    pub fn snapshots_from_metadata(metadata: &BTreeMap<String, String>) -> Vec<ProviderSnapshot> {
        let Some(raw_ids) = metadata.get("providers") else {
            return Vec::new();
        };
        raw_ids
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(|id| ProviderSnapshot {
                provider_id: id.to_string(),
                healthy: metadata
                    .get(&format!("provider.{id}.healthy"))
                    .map(|value| value == "true")
                    .unwrap_or(true),
                latency_ms: metadata
                    .get(&format!("provider.{id}.latency_ms"))
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(100),
                unit_cost: metadata
                    .get(&format!("provider.{id}.unit_cost"))
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(100),
                trust_score: metadata
                    .get(&format!("provider.{id}.trust_score"))
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(50),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticky_prefers_previous_provider_when_healthy() {
        let selected = ProviderSelector::select(
            ProviderSelectionStrategy::Sticky,
            Some("provider.b"),
            &[
                ProviderSnapshot {
                    provider_id: "provider.a".into(),
                    healthy: true,
                    latency_ms: 10,
                    unit_cost: 10,
                    trust_score: 10,
                },
                ProviderSnapshot {
                    provider_id: "provider.b".into(),
                    healthy: true,
                    latency_ms: 90,
                    unit_cost: 90,
                    trust_score: 90,
                },
            ],
        );
        assert_eq!(selected.as_deref(), Some("provider.b"));
    }

    #[test]
    fn cost_first_prefers_lowest_cost_provider() {
        let selected = ProviderSelector::select(
            ProviderSelectionStrategy::CostFirst,
            None,
            &[
                ProviderSnapshot {
                    provider_id: "provider.a".into(),
                    healthy: true,
                    latency_ms: 10,
                    unit_cost: 90,
                    trust_score: 10,
                },
                ProviderSnapshot {
                    provider_id: "provider.b".into(),
                    healthy: true,
                    latency_ms: 90,
                    unit_cost: 10,
                    trust_score: 90,
                },
            ],
        );
        assert_eq!(selected.as_deref(), Some("provider.b"));
    }
}
