//! Host and deterministic random providers.
//!
//! `HostRandomProvider` uses the operating system CSPRNG through `getrandom`.
//! `DeterministicRandomProvider` is intentionally test/replay-only and uses a
//! bounded opaque stream state. Both providers share the same Command boundary;
//! neither exposes a native RNG object to callers.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine;
use macaca_proto::{
    CapabilityId, CleanupPolicy, RandomEntropyHealth, RandomProviderSnapshot, ServiceCallResult,
    ServiceCommand, ServiceError, ServiceHealth, ServiceResult, TraceContext,
    FOUNDATION_RANDOM_COMMANDS, FOUNDATION_RANDOM_SERVICE_ID,
};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::service_contract::RandomService;

const MAX_BYTES: u32 = 65_536;
const MAX_TOKEN: u32 = 512;

/// OS-backed cryptographically secure random provider.
#[derive(Debug, Default)]
pub struct HostRandomProvider;

/// Deterministic provider whose seed remains an opaque reference.
#[derive(Debug, Default)]
pub struct DeterministicRandomProvider {
    streams: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

#[async_trait]
impl RandomService for HostRandomProvider {
    fn descriptor(&self) -> macaca_proto::ServiceDescriptor {
        descriptor("host-csprng", true)
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        dispatch(command, None)
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> RandomProviderSnapshot {
        snapshot("host-csprng", true, BTreeMap::new())
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}

#[async_trait]
impl RandomService for DeterministicRandomProvider {
    fn descriptor(&self) -> macaca_proto::ServiceDescriptor {
        descriptor("deterministic-test", true)
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        let stream_key = command
            .payload
            .get("stream_id")
            .and_then(Value::as_str)
            .unwrap_or("default");
        let mut streams = self
            .streams
            .lock()
            .map_err(|_| ServiceError::AdapterFailure("stream lock poisoned".into()))?;
        let stream = streams.entry(stream_key.into()).or_default();
        dispatch_with_bytes(command, Some(stream), trace)
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> RandomProviderSnapshot {
        let hashes = self
            .streams
            .lock()
            .map(|streams| {
                streams
                    .iter()
                    .map(|(id, value)| {
                        (
                            format!("stream:{:016x}", stable_hash(id)),
                            value.len().to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        snapshot("deterministic-test", true, hashes)
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        self.streams
            .lock()
            .map(|mut value| value.clear())
            .map_err(|_| ServiceError::AdapterFailure("stream lock poisoned".into()))
    }
}

fn dispatch(
    command: ServiceCommand,
    stream: Option<&mut Vec<u8>>,
) -> ServiceResult<ServiceCallResult> {
    let trace = command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)?;
    dispatch_with_bytes(command, stream, trace)
}

fn dispatch_with_bytes(
    command: ServiceCommand,
    mut stream: Option<&mut Vec<u8>>,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let name = command.name.as_str();
    let payload = command.payload;
    let result = match name {
        "random.bytes" | "random.fill" | "random.nonce" => {
            let length = payload
                .get("length")
                .or_else(|| payload.get("byte_length"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let bytes = generate_bytes(length, stream.as_deref_mut())?;
            serde_json::json!({"status":"success","data":encode(bytes)})
        }
        "random.integer" => {
            let min = payload
                .get("min_inclusive")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let max = payload
                .get("max_exclusive")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if min >= max {
                return Err(ServiceError::InvalidArgument("invalid random range".into()));
            }
            let value = bounded_integer(min, max, stream.as_deref_mut())?;
            serde_json::json!({"status":"success","data":value})
        }
        "random.uuid_v4" => {
            let count = payload
                .get("count")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .min(32);
            let mut values = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let mut bytes: [u8; 16] = generate_bytes(16, stream.as_deref_mut())?
                    .try_into()
                    .map_err(|_| {
                        ServiceError::AdapterFailure("random UUID byte conversion failed".into())
                    })?;
                bytes[6] = (bytes[6] & 0x0f) | 0x40;
                bytes[8] = (bytes[8] & 0x3f) | 0x80;
                values.push(Uuid::from_bytes(bytes).to_string());
            }
            serde_json::json!({"status":"success","data":values})
        }
        "random.token" => {
            let length = payload
                .get("char_length")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let bytes = generate_bytes(length.min(MAX_TOKEN), stream.as_deref_mut())?;
            serde_json::json!({"status":"success","data":encode(bytes)})
        }
        "random.entropy_health" | "random.provider_capabilities" => {
            serde_json::json!({"status":"success","provider_class":"host-csprng","max_bytes_per_request":MAX_BYTES})
        }
        "random.test_stream_create" | "random.test_stream_bytes" => {
            if stream.is_some() {
                serde_json::json!({"status":"success","data":encode(generate_bytes(16, stream.as_deref_mut())?)})
            } else {
                serde_json::json!({"status":"deterministic_not_allowed"})
            }
        }
        _ => return Err(ServiceError::UnsupportedCommand(name.into())),
    };
    info!(service_id = FOUNDATION_RANDOM_SERVICE_ID, command = name,
        trace_id = %trace.trace_id, "random service command completed");
    Ok(ServiceCallResult {
        output: result,
        trace,
        status: "ok".into(),
        metadata: Default::default(),
        cleanup_hint: Some(CleanupPolicy::None),
    })
}

fn generate_bytes(length: u32, stream: Option<&mut Vec<u8>>) -> Result<Vec<u8>, ServiceError> {
    if length == 0 || length > MAX_BYTES {
        return Err(ServiceError::InvalidArgument(
            "random length exceeds policy".into(),
        ));
    }
    let mut output = vec![0; length as usize];
    if let Some(state) = stream {
        for (index, byte) in output.iter_mut().enumerate() {
            *byte = state
                .get(index % state.len().max(1))
                .copied()
                .unwrap_or(index as u8)
                .wrapping_add(index as u8);
        }
        state.extend_from_slice(&output);
    } else {
        getrandom::getrandom(&mut output).map_err(|error| {
            ServiceError::HealthCheckFailed(format!("entropy unavailable: {error}"))
        })?;
    }
    Ok(output)
}

fn generate_fixed(stream: Option<&mut Vec<u8>>) -> Result<[u8; 8], ServiceError> {
    generate_bytes(8, stream).map(|value| value.try_into().unwrap_or([0; 8]))
}

/// Generate a bounded integer using rejection sampling to avoid modulo bias.
fn bounded_integer(
    min_inclusive: i64,
    max_exclusive: i64,
    mut stream: Option<&mut Vec<u8>>,
) -> Result<i64, ServiceError> {
    let span = (max_exclusive - min_inclusive) as u64;
    let threshold = u64::MAX - (u64::MAX % span);
    loop {
        let candidate = u64::from_le_bytes(generate_fixed(stream.as_deref_mut())?);
        if candidate < threshold {
            return Ok(min_inclusive + (candidate % span) as i64);
        }
    }
}

fn encode(bytes: Vec<u8>) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Create a replay-safe Memento using only provider class and bounded stream facts.
fn snapshot(
    provider_class: &str,
    entropy_available: bool,
    stream_position_hashes: BTreeMap<String, String>,
) -> RandomProviderSnapshot {
    RandomProviderSnapshot {
        descriptor_hash: format!("foundation-random-{provider_class}-v1"),
        provider_class: provider_class.into(),
        health: RandomEntropyHealth {
            provider_class: provider_class.into(),
            entropy_available,
            blocking_risk: false,
            max_bytes_per_request: MAX_BYTES,
            unavailable_reason: None,
        },
        stream_position_hashes,
    }
}

/// Hash opaque stream identifiers before snapshotting so callers never see handles.
fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(byte as u64)
    })
}

fn descriptor(provider: &str, healthy: bool) -> macaca_proto::ServiceDescriptor {
    let mut descriptor = macaca_proto::ServiceDescriptor::new(
        macaca_proto::KernelServiceId::new(FOUNDATION_RANDOM_SERVICE_ID),
        macaca_proto::ServiceType::new("foundation.random"),
        macaca_proto::TraceSchemaRef::new("macaca.trace.foundation.random.v1"),
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
    descriptor.capabilities = FOUNDATION_RANDOM_COMMANDS
        .iter()
        .map(|name| {
            macaca_proto::ServiceCapability::new(CapabilityId::new(*name), "random command")
        })
        .collect();
    descriptor
}

#[cfg(test)]
mod tests {
    use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

    use super::{DeterministicRandomProvider, HostRandomProvider};
    use crate::service_contract::RandomService;

    fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
        ServiceCommand::with_trace(
            ServiceCommandName::new(name),
            payload,
            TraceContext::new(format!("trace-{name}")),
        )
    }

    #[tokio::test]
    async fn host_provider_generates_bounded_bytes_and_bias_free_integer() {
        let provider = HostRandomProvider;
        let bytes = provider
            .call(command("random.bytes", serde_json::json!({"length": 16})))
            .await
            .unwrap();
        assert_eq!(bytes.output["status"], "success");
        let integer = provider
            .call(command(
                "random.integer",
                serde_json::json!({"min_inclusive": 3, "max_exclusive": 7}),
            ))
            .await
            .unwrap();
        let value = integer.output["data"].as_i64().unwrap();
        assert!((3..7).contains(&value));
    }

    #[tokio::test]
    async fn deterministic_provider_preserves_stream_state_without_seed_echo() {
        let provider = DeterministicRandomProvider::default();
        let first = provider
            .call(command(
                "random.test_stream_bytes",
                serde_json::json!({"stream_id":"stream:one", "length": 16, "seed":"raw-seed"}),
            ))
            .await
            .unwrap();
        let second = provider
            .call(command(
                "random.test_stream_bytes",
                serde_json::json!({"stream_id":"stream:one", "length": 16}),
            ))
            .await
            .unwrap();
        assert_ne!(first.output["data"], second.output["data"]);
        assert!(!first.output.to_string().contains("raw-seed"));
    }

    #[tokio::test]
    async fn unsupported_and_invalid_requests_do_not_fallback() {
        let provider = HostRandomProvider;
        let invalid = provider
            .call(command("random.bytes", serde_json::json!({"length": 0})))
            .await
            .unwrap_err();
        assert!(invalid.to_string().contains("random length"));
        let unsupported = provider
            .call(command("random.not_declared", serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(unsupported.to_string().contains("unsupported"));
    }
}
