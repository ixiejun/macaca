use std::future::Future;
use std::time::{Duration, Instant};

use macaca_proto::{MacacaError, MacacaResult};
use tokio::sync::RwLock;

use super::config::MemoryProviderResilienceConfig;

/// Mutable circuit breaker state shared by provider adapters.
///
/// The circuit breaker is intentionally small and local to the provider
/// instance. It does not try to implement a global policy engine; it only keeps
/// repeated external failures from overwhelming a single provider path.
#[derive(Debug, Clone)]
pub struct MemoryProviderCircuitBreaker {
    inner: std::sync::Arc<RwLock<MemoryProviderCircuitState>>,
    failure_threshold: u32,
    cooldown: Duration,
}

/// Internal state for the circuit breaker.
#[derive(Debug, Clone)]
struct MemoryProviderCircuitState {
    failure_count: u32,
    open_until: Option<Instant>,
    last_error: Option<String>,
}

impl MemoryProviderCircuitBreaker {
    /// Create a circuit breaker from provider resilience configuration.
    pub fn new(config: &MemoryProviderResilienceConfig) -> Self {
        Self {
            inner: std::sync::Arc::new(RwLock::new(MemoryProviderCircuitState {
                failure_count: 0,
                open_until: None,
                last_error: None,
            })),
            failure_threshold: config.circuit_failure_threshold,
            cooldown: Duration::from_millis(config.circuit_cooldown_ms),
        }
    }

    /// Return whether the circuit is currently open.
    pub async fn is_open(&self) -> bool {
        let mut guard = self.inner.write().await;
        if let Some(until) = guard.open_until {
            if Instant::now() < until {
                return true;
            }
            guard.open_until = None;
            guard.failure_count = 0;
            guard.last_error = None;
        }
        false
    }

    /// Record a successful call and close the circuit if needed.
    pub async fn record_success(&self) {
        let mut guard = self.inner.write().await;
        guard.failure_count = 0;
        guard.open_until = None;
        guard.last_error = None;
    }

    /// Record a failure and open the circuit if the threshold is exceeded.
    pub async fn record_failure(&self, error: impl Into<String>) {
        let mut guard = self.inner.write().await;
        guard.failure_count = guard.failure_count.saturating_add(1);
        guard.last_error = Some(error.into());
        if guard.failure_count >= self.failure_threshold {
            guard.open_until = Some(Instant::now() + self.cooldown);
        }
    }

    /// Return the last error snapshot for diagnostics.
    pub async fn last_error(&self) -> Option<String> {
        self.inner.read().await.last_error.clone()
    }
}

/// Standard resilience wrapper used by remote and MCP provider adapters.
///
/// This wrapper handles timeout budgeting, payload-size guarding, retry policy,
/// and circuit breaking. It deliberately stays generic so the same decorator
/// can wrap different transports without embedding transport-specific logic.
#[derive(Debug, Clone)]
pub struct MemoryProviderResilience {
    timeout: Duration,
    payload_limit_bytes: usize,
    retry_count: u32,
    circuit: MemoryProviderCircuitBreaker,
}

impl MemoryProviderResilience {
    /// Create a resilience wrapper from provider configuration.
    pub fn new(config: &MemoryProviderResilienceConfig) -> Self {
        Self {
            timeout: Duration::from_millis(config.timeout_ms),
            payload_limit_bytes: config.payload_limit_bytes,
            retry_count: config.retry_count,
            circuit: MemoryProviderCircuitBreaker::new(config),
        }
    }

    /// Return the configured timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Guard a transport call with payload checks, retries, timeouts, and a circuit breaker.
    pub async fn execute<T, F, Fut>(
        &self,
        operation: &'static str,
        payload_bytes: usize,
        call: F,
    ) -> MacacaResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = MacacaResult<T>>,
    {
        if payload_bytes > self.payload_limit_bytes {
            return Err(MacacaError::Memory(format!(
                "{operation} payload exceeded provider limit: {payload_bytes} > {}",
                self.payload_limit_bytes
            )));
        }

        if self.circuit.is_open().await {
            let message = self
                .circuit
                .last_error()
                .await
                .unwrap_or_else(|| "circuit open".into());
            return Err(MacacaError::Memory(format!(
                "{operation} blocked by open circuit: {message}"
            )));
        }

        let attempts = self.retry_count.saturating_add(1);
        let mut last_error = None;

        for attempt in 0..attempts {
            let result = tokio::time::timeout(self.timeout, call()).await;
            match result {
                Ok(Ok(value)) => {
                    self.circuit.record_success().await;
                    return Ok(value);
                }
                Ok(Err(err)) => {
                    let message = err.to_string();
                    self.circuit.record_failure(message.clone()).await;
                    last_error = Some(message);
                }
                Err(_) => {
                    let message = format!("{operation} timed out after {:?}", self.timeout);
                    self.circuit.record_failure(message.clone()).await;
                    last_error = Some(message);
                }
            }

            if attempt + 1 < attempts {
                tokio::time::sleep(Duration::from_millis(50 * u64::from(attempt + 1))).await;
            }
        }

        Err(MacacaError::Memory(format!(
            "{operation} failed after {attempts} attempt(s): {}",
            last_error.unwrap_or_else(|| "unknown error".into())
        )))
    }
}

/// Redact obvious secret values from diagnostics and error strings.
pub fn redact_text(input: &str, secret_markers: &[String]) -> String {
    let mut output = input.to_string();
    for marker in secret_markers {
        if !marker.is_empty() {
            output = output.replace(marker, "***redacted***");
        }
    }
    output
}
