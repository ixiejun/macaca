//! Gate Strategy implementation for local Heartbeat wake processing.
//!
//! Gates are generic policy/resource checks. They never inspect application
//! names, workflow names, provider names, model names, driver names, gateway
//! names, chain names, payment names, or business-domain strings.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Timelike, Utc};
use macaca_proto::{HeartbeatGateDecision, HeartbeatGateKind, HeartbeatWakeCommand};

use super::{
    memento::LocalHeartbeatState, DEFAULT_ACTIVE_END_HOUR_UTC, DEFAULT_ACTIVE_START_HOUR_UTC,
    DEFAULT_COOLDOWN_MS,
};

const RESOURCE_UNITS_KEY: &str = "resource_units";
const BUDGET_UNITS_KEY: &str = "budget_units";
const DEFAULT_RESOURCE_CAPACITY_UNITS: u64 = 1_000;
const DEFAULT_BUDGET_CAPACITY_UNITS: u64 = 1_000;

#[derive(Clone)]
pub(super) struct DefaultHeartbeatGateStrategy {
    active_start_hour_utc: u32,
    active_end_hour_utc: u32,
    cooldown_ms: i64,
    resource_capacity_units: u64,
    budget_capacity_units: u64,
}

impl Default for DefaultHeartbeatGateStrategy {
    fn default() -> Self {
        Self {
            active_start_hour_utc: DEFAULT_ACTIVE_START_HOUR_UTC,
            active_end_hour_utc: DEFAULT_ACTIVE_END_HOUR_UTC,
            cooldown_ms: DEFAULT_COOLDOWN_MS,
            resource_capacity_units: DEFAULT_RESOURCE_CAPACITY_UNITS,
            budget_capacity_units: DEFAULT_BUDGET_CAPACITY_UNITS,
        }
    }
}

impl DefaultHeartbeatGateStrategy {
    pub(super) fn evaluate(
        &self,
        state: &mut LocalHeartbeatState,
        command: &HeartbeatWakeCommand,
    ) -> Vec<HeartbeatGateDecision> {
        let now = Utc::now();
        let active_hours_allowed = self.in_active_hours(now);
        let cooldown_allowed = state
            .last_accepted_by_scope
            .get(&command.wake_scope_key)
            .map(|last| now.signed_duration_since(*last).num_milliseconds() >= self.cooldown_ms)
            .unwrap_or(true);
        let busy_allowed = !state.pending_by_scope.contains_key(&command.wake_scope_key);
        let resource_units = metadata_units(&command.metadata, RESOURCE_UNITS_KEY);
        let budget_units = metadata_units(&command.metadata, BUDGET_UNITS_KEY);
        let resource_allowed = resource_units <= self.resource_capacity_units;
        let budget_allowed = budget_units <= self.budget_capacity_units;
        let gates = vec![
            gate(
                HeartbeatGateKind::ActiveHours,
                active_hours_allowed,
                "active_hours",
                None,
            ),
            gate(
                HeartbeatGateKind::Cooldown,
                cooldown_allowed,
                "cooldown",
                if cooldown_allowed {
                    None
                } else {
                    state
                        .last_accepted_by_scope
                        .get(&command.wake_scope_key)
                        .map(|last| *last + Duration::milliseconds(self.cooldown_ms))
                },
            ),
            gate(HeartbeatGateKind::Busy, busy_allowed, "busy", None),
            gate(
                HeartbeatGateKind::Resource,
                resource_allowed,
                "resource",
                None,
            ),
            gate(HeartbeatGateKind::Budget, budget_allowed, "budget", None),
            gate(
                HeartbeatGateKind::ProviderHealth,
                true,
                "provider_health",
                None,
            ),
        ];
        if gates.iter().all(|gate| gate.allowed) {
            state
                .last_accepted_by_scope
                .insert(command.wake_scope_key.clone(), now);
        }
        gates
    }

    fn in_active_hours(&self, now: DateTime<Utc>) -> bool {
        if self.active_start_hour_utc == self.active_end_hour_utc {
            return true;
        }
        let hour = now.hour();
        if self.active_start_hour_utc < self.active_end_hour_utc {
            hour >= self.active_start_hour_utc && hour < self.active_end_hour_utc
        } else {
            hour >= self.active_start_hour_utc || hour < self.active_end_hour_utc
        }
    }
}

fn gate(
    kind: HeartbeatGateKind,
    allowed: bool,
    reason_code: &'static str,
    next_eligible_at: Option<DateTime<Utc>>,
) -> HeartbeatGateDecision {
    HeartbeatGateDecision {
        gate: kind,
        allowed,
        reason_code: reason_code.into(),
        next_eligible_at,
        metadata: BTreeMap::new(),
    }
}

fn metadata_units(metadata: &BTreeMap<String, String>, key: &str) -> u64 {
    metadata
        .get(key)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}
