//! Host and deterministic providers for provider-neutral time commands.
//!
//! Timer records are bounded metadata, not host timer handles.  A caller can
//! inspect or cancel a record through the same service boundary while host
//! scheduling remains replaceable by a remote or plugin provider.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TimeClockHealth, TimeProviderSnapshot, TraceSchemaRef, FOUNDATION_TIME_COMMANDS,
    FOUNDATION_TIME_SERVICE_ID,
};
use tracing::info;

use crate::service_contract::TimeService;
use crate::time_conversion::{
    calendar_convert, convert_timezone, format_time, parse_time, resolve_timezone,
};

const MAX_TIMER_DURATION_MS: i128 = 86_400_000;
const MAX_ACTIVE_TIMERS: usize = 256;

/// Built-in host adapter using UTC wall time and process-local monotonic ticks.
#[derive(Debug)]
pub struct HostTimeProvider {
    origin: Instant,
    timers: TimerStore,
}

/// Deterministic frozen clock for test and replay compositions only.
#[derive(Debug)]
pub struct FrozenTimeProvider {
    epoch_millis: Arc<Mutex<i128>>,
    timers: TimerStore,
}

type TimerStore = Arc<Mutex<BTreeMap<String, TimerRecord>>>;

#[derive(Debug, Clone)]
struct TimerRecord {
    reservation_id: String,
    due_epoch_millis: i128,
    exactness: String,
    state: &'static str,
}

impl Default for HostTimeProvider {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
            timers: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl FrozenTimeProvider {
    /// Create a deterministic test clock without accessing the host clock.
    pub fn new(epoch_millis: i128) -> Self {
        Self {
            epoch_millis: Arc::new(Mutex::new(epoch_millis)),
            timers: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Advance the deterministic test/replay clock without touching host time.
    pub fn advance_millis(&self, duration_millis: i128) -> ServiceResult<()> {
        let mut epoch = self
            .epoch_millis
            .lock()
            .map_err(|_| ServiceError::AdapterFailure("frozen clock lock poisoned".into()))?;
        *epoch = epoch
            .checked_add(duration_millis)
            .ok_or_else(|| ServiceError::InvalidArgument("frozen clock overflow".into()))?;
        info!(
            service_id = FOUNDATION_TIME_SERVICE_ID,
            advanced_millis = duration_millis,
            "time_pack_frozen_clock_advanced"
        );
        Ok(())
    }
}

#[async_trait]
impl TimeService for HostTimeProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor("host-clock", true)
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        dispatch(
            command,
            Utc::now().timestamp_millis() as i128,
            self.origin.elapsed().as_nanos(),
            &self.timers,
            true,
            "host-clock",
        )
    }
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> TimeProviderSnapshot {
        snapshot("host-clock", &self.timers, true, true)
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        clear_timers(&self.timers)
    }
}

#[async_trait]
impl TimeService for FrozenTimeProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor("frozen-test-clock", true)
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let epoch_millis = *self
            .epoch_millis
            .lock()
            .map_err(|_| ServiceError::AdapterFailure("frozen clock lock poisoned".into()))?;
        dispatch(
            command,
            epoch_millis,
            0,
            &self.timers,
            false,
            "frozen-test-clock",
        )
    }
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> TimeProviderSnapshot {
        snapshot("frozen-test-clock", &self.timers, true, false)
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        clear_timers(&self.timers)
    }
}

fn dispatch(
    command: ServiceCommand,
    now: i128,
    monotonic: u128,
    timers: &TimerStore,
    exact: bool,
    provider_class: &str,
) -> ServiceResult<ServiceCallResult> {
    let trace = command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)?;
    let payload = command.payload;
    let output = match command.name.as_str() {
        "time.now" => {
            serde_json::json!({"status":"success","epoch_millis":now,"clock_source":"wall_clock","timezone_data_version":"host"})
        }
        "time.monotonic_now" => {
            serde_json::json!({"status":"success","ticks":monotonic,"unit":"nanos","clock_source":"monotonic"})
        }
        "time.clock_health" => {
            serde_json::json!({"status":"success","wall_clock_available":true,"monotonic_available":true,"max_timer_duration_ms":MAX_TIMER_DURATION_MS,"supports_exact_timers":exact})
        }
        "time.duration_between" => duration_between(&payload)?,
        "time.add_duration" => add_duration(&payload)?,
        "time.resolve_timezone" => resolve_timezone(&payload)?,
        "time.convert_timezone" => convert_timezone(&payload)?,
        "time.calendar_convert" => calendar_convert(&payload)?,
        "time.format" => format_time(&payload)?,
        "time.parse" => parse_time(&payload)?,
        "time.create_timer" => create_timer(&payload, now, timers, exact)?,
        "time.cancel_timer" => cancel_timer(&payload, timers)?,
        "time.inspect_timer" => inspect_timer(&payload, now, timers)?,
        "time.evaluate_deadline" => evaluate_deadline(&payload, now)?,
        name => return Err(ServiceError::UnsupportedCommand(name.into())),
    };
    info!(service_id = FOUNDATION_TIME_SERVICE_ID, command = %command.name,
        trace_id = %trace.trace_id, "time service command completed");
    Ok(ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata: replay_metadata(command.name.as_str(), provider_class, exact),
        cleanup_hint: Some(CleanupPolicy::OnStop),
    })
}

/// Provide bounded decision facts for generic router replay without exposing
/// payloads, timer handles, or provider-internal state. The runtime host owns
/// the allowlist that decides which of these facts reaches the audit sink.
fn replay_metadata(command: &str, provider_class: &str, exact: bool) -> BTreeMap<String, String> {
    let mut metadata =
        BTreeMap::from([("replay.provider_class".into(), provider_class.to_string())]);
    match command {
        "time.now" | "time.evaluate_deadline" => {
            metadata.insert("replay.clock_source".into(), "wall_clock".into());
            metadata.insert(
                "replay.timezone_data_version".into(),
                "fixed-offset-v1".into(),
            );
        }
        "time.monotonic_now" => {
            metadata.insert("replay.clock_source".into(), "monotonic".into());
            metadata.insert("replay.monotonic_unit".into(), "nanos".into());
        }
        "time.resolve_timezone" | "time.convert_timezone" | "time.parse" => {
            metadata.insert(
                "replay.timezone_data_version".into(),
                "fixed-offset-v1".into(),
            );
        }
        "time.clock_health" if !exact => {
            metadata.insert("replay.clock_source".into(), "frozen".into());
        }
        _ => {}
    }
    metadata
}

fn duration_between(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let start = payload
        .pointer("/start/epoch_millis")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("start instant is required".into()))?;
    let end = payload
        .pointer("/end/epoch_millis")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("end instant is required".into()))?;
    Ok(serde_json::json!({"status":"success","millis":i128::from(end)-i128::from(start)}))
}

fn add_duration(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let instant = payload
        .pointer("/instant/epoch_millis")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("instant is required".into()))?;
    let duration = payload
        .pointer("/duration/millis")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("duration is required".into()))?;
    Ok(
        serde_json::json!({"status":"success","epoch_millis":i128::from(instant)+i128::from(duration)}),
    )
}

fn create_timer(
    payload: &serde_json::Value,
    now: i128,
    timers: &TimerStore,
    exact: bool,
) -> ServiceResult<serde_json::Value> {
    let duration = payload
        .pointer("/duration/millis")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("timer duration is required".into()))?
        as i128;
    if !(1..=MAX_TIMER_DURATION_MS).contains(&duration) {
        return Err(ServiceError::InvalidArgument(
            "timer duration exceeds policy".into(),
        ));
    }
    let mut entries = timers
        .lock()
        .map_err(|_| ServiceError::AdapterFailure("timer lock poisoned".into()))?;
    if entries.len() >= MAX_ACTIVE_TIMERS {
        return Err(ServiceError::DisabledByPolicy(
            "active timer quota exceeded".into(),
        ));
    }
    // The timer map is the provider-local reservation ledger. Every terminal
    // transition removes one record, releasing its reservation exactly once.
    let id = format!("timer-{}", entries.len() + 1);
    let reservation_id = format!("reservation-{id}");
    let exactness = payload
        .get("exactness")
        .and_then(|v| v.as_str())
        .unwrap_or("inexact_allowed");
    if exactness == "exact_required" && !exact {
        return Ok(serde_json::json!({"status":"unsupported","reason":"exact_timer_unsupported"}));
    }
    entries.insert(
        id.clone(),
        TimerRecord {
            reservation_id: reservation_id.clone(),
            due_epoch_millis: now + duration,
            exactness: exactness.into(),
            state: "active",
        },
    );
    info!(service_id = FOUNDATION_TIME_SERVICE_ID,
        timer_id_hash = %stable_timer_hash(&id),
        reservation_id_hash = %stable_timer_hash(&reservation_id),
        "time_pack_timer_created");
    Ok(
        serde_json::json!({"status":"success","timer_id":id,"exact":exact && exactness != "inexact_allowed"}),
    )
}

fn cancel_timer(
    payload: &serde_json::Value,
    timers: &TimerStore,
) -> ServiceResult<serde_json::Value> {
    let id = payload
        .pointer("/timer/timer_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServiceError::InvalidArgument("timer id is required".into()))?;
    let removed = timers
        .lock()
        .map_err(|_| ServiceError::AdapterFailure("timer lock poisoned".into()))?
        .remove(id);
    if let Some(record) = &removed {
        info!(service_id = FOUNDATION_TIME_SERVICE_ID,
            timer_id_hash = %stable_timer_hash(id),
            reservation_id_hash = %stable_timer_hash(&record.reservation_id),
            "time_pack_timer_cancelled");
    }
    Ok(if removed.is_some() {
        serde_json::json!({"status":"success","state":"cancelled"})
    } else {
        serde_json::json!({"status":"timer_not_found"})
    })
}

fn inspect_timer(
    payload: &serde_json::Value,
    now: i128,
    timers: &TimerStore,
) -> ServiceResult<serde_json::Value> {
    let id = payload
        .pointer("/timer/timer_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServiceError::InvalidArgument("timer id is required".into()))?;
    let mut entries = timers
        .lock()
        .map_err(|_| ServiceError::AdapterFailure("timer lock poisoned".into()))?;
    let entry = entries.get(id).cloned();
    if entry
        .as_ref()
        .is_some_and(|record| now >= record.due_epoch_millis)
    {
        if let Some(record) = entries.remove(id) {
            info!(service_id = FOUNDATION_TIME_SERVICE_ID,
                timer_id_hash = %stable_timer_hash(id),
                reservation_id_hash = %stable_timer_hash(&record.reservation_id),
                "time_pack_timer_fired");
        }
    }
    Ok(match entry {
        Some(record) => {
            serde_json::json!({"status":"success","state":if now >= record.due_epoch_millis {"fired"} else {record.state},"exactness":record.exactness})
        }
        None => serde_json::json!({"status":"timer_not_found"}),
    })
}

fn evaluate_deadline(payload: &serde_json::Value, now: i128) -> ServiceResult<serde_json::Value> {
    let deadline = payload
        .pointer("/deadline/deadline/epoch_millis")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("deadline is required".into()))?
        as i128;
    Ok(
        serde_json::json!({"status":"success","expired":now >= deadline,"clock_source":"wall_clock"}),
    )
}

fn clear_timers(timers: &TimerStore) -> ServiceResult<()> {
    let mut entries = timers
        .lock()
        .map_err(|_| ServiceError::AdapterFailure("timer lock poisoned".into()))?;
    let released_reservations = entries.len();
    entries.clear();
    info!(
        service_id = FOUNDATION_TIME_SERVICE_ID,
        released_reservations, "time_pack_timer_resources_released_on_shutdown"
    );
    Ok(())
}

fn stable_timer_hash(value: &str) -> String {
    let digest = value.bytes().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(byte as u64)
    });
    format!("{digest:016x}")
}

/// Build a bounded Memento from provider metadata and hashed timer state only.
fn snapshot(
    provider_class: &str,
    timers: &TimerStore,
    monotonic_available: bool,
    supports_exact_timers: bool,
) -> TimeProviderSnapshot {
    let timer_state_hashes = timers
        .lock()
        .map(|entries| {
            entries
                .iter()
                .map(|(id, record)| {
                    let digest = stable_timer_hash(&format!("{id}:{}", record.state));
                    (format!("timer:{digest}"), record.state.to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    TimeProviderSnapshot {
        descriptor_hash: format!("foundation-time-{provider_class}-v1"),
        provider_class: provider_class.into(),
        health: TimeClockHealth {
            provider_class: provider_class.into(),
            wall_clock_available: true,
            monotonic_available,
            timezone_data_version: Some("fixed-offset-v1".into()),
            locale_data_available: true,
            max_timer_duration_ms: MAX_TIMER_DURATION_MS as u64,
            unavailable_reason: if supports_exact_timers {
                None
            } else {
                Some("exact_timer_unsupported".into())
            },
        },
        timer_state_hashes,
    }
}

fn descriptor(provider: &str, healthy: bool) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(FOUNDATION_TIME_SERVICE_ID),
        ServiceType::new("foundation.time"),
        TraceSchemaRef::new("macaca.trace.foundation.time.v1"),
    );
    descriptor.health = if healthy {
        ServiceHealth::Healthy
    } else {
        ServiceHealth::Unavailable {
            reason: provider.into(),
        }
    };
    descriptor
        .metadata
        .insert("provider_class".into(), provider.into());
    descriptor
        .metadata
        .insert("max_active_timers".into(), MAX_ACTIVE_TIMERS.to_string());
    descriptor.capabilities = FOUNDATION_TIME_COMMANDS
        .iter()
        .map(|name| ServiceCapability::new(CapabilityId::new(*name), "time command"))
        .collect();
    descriptor
}

#[cfg(test)]
#[path = "local_provider_tests.rs"]
mod tests;
