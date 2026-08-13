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
    TraceSchemaRef, FOUNDATION_TIME_COMMANDS, FOUNDATION_TIME_SERVICE_ID,
};
use tracing::info;

use crate::service_contract::TimeService;

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
    epoch_millis: i128,
    timers: TimerStore,
}

type TimerStore = Arc<Mutex<BTreeMap<String, TimerRecord>>>;

#[derive(Debug, Clone)]
struct TimerRecord {
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
            epoch_millis,
            timers: Arc::new(Mutex::new(BTreeMap::new())),
        }
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
        )
    }
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
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
        dispatch(command, self.epoch_millis, 0, &self.timers, false)
    }
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
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
        "time.resolve_timezone"
        | "time.convert_timezone"
        | "time.calendar_convert"
        | "time.format"
        | "time.parse" => {
            serde_json::json!({"status":"unsupported","reason":"timezone_calendar_locale_adapter_not_installed"})
        }
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
        metadata: Default::default(),
        cleanup_hint: Some(CleanupPolicy::OnStop),
    })
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
    let id = format!("timer-{}", entries.len() + 1);
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
            due_epoch_millis: now + duration,
            exactness: exactness.into(),
            state: "active",
        },
    );
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
    let entry = timers
        .lock()
        .map_err(|_| ServiceError::AdapterFailure("timer lock poisoned".into()))?
        .get(id)
        .cloned();
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
    timers
        .lock()
        .map(|mut value| value.clear())
        .map_err(|_| ServiceError::AdapterFailure("timer lock poisoned".into()))
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
mod tests {
    use crate::{FrozenTimeProvider, HostTimeProvider, TimeService};
    use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
    fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
        ServiceCommand::with_trace(
            ServiceCommandName::new(name),
            payload,
            TraceContext::new("trace-time"),
        )
    }
    #[tokio::test]
    async fn frozen_clock_is_deterministic_and_timer_lifecycle_is_bounded() {
        let provider = FrozenTimeProvider::new(1_000);
        assert_eq!(
            provider
                .call(command("time.now", serde_json::json!({})))
                .await
                .unwrap()
                .output["epoch_millis"],
            1_000
        );
        let created = provider
            .call(command(
                "time.create_timer",
                serde_json::json!({"duration":{"millis":10},"exactness":"inexact_allowed"}),
            ))
            .await
            .unwrap();
        let id = created.output["timer_id"].clone();
        assert_eq!(
            provider
                .call(command(
                    "time.cancel_timer",
                    serde_json::json!({"timer":{"timer_id":id}})
                ))
                .await
                .unwrap()
                .output["state"],
            "cancelled"
        );
    }
    #[tokio::test]
    async fn host_provider_rejects_invalid_timer_without_fallback() {
        let error = HostTimeProvider::default()
            .call(command(
                "time.create_timer",
                serde_json::json!({"duration":{"millis":0}}),
            ))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("duration"));
    }
}
