//! Prometheus metrics for Macaca Agent OS.
//!
//! Exposes a global `REGISTRY` and typed metric statics. Any crate can call
//! the convenience functions (`record_llm_request`, etc.) to record data.
//! The `/metrics` HTTP endpoint is registered in `lib.rs`.

use once_cell::sync::Lazy;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};

/// Global Prometheus registry (separate from the default registry so we
/// control exactly which metrics are exposed).
pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// Total LLM API requests — labels: `model`, `status` (`success` | `error`).
pub static LLM_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("llm_requests_total", "Total LLM API requests");
    let counter = IntCounterVec::new(opts, &["model", "status"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

/// LLM request duration histogram — labels: `model`.
pub static LLM_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "llm_request_duration_seconds",
        "LLM request duration in seconds",
    );
    let hist = HistogramVec::new(opts, &["model"]).unwrap();
    REGISTRY.register(Box::new(hist.clone())).unwrap();
    hist
});

/// Total LLM tokens used — labels: `model`, `type` (`prompt` | `completion`).
pub static LLM_TOKENS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("llm_tokens_total", "Total LLM tokens used");
    let counter = IntCounterVec::new(opts, &["model", "type"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

/// Total tasks delegated to agents — labels: `agent`.
pub static TASK_DELEGATED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("task_delegated_total", "Total tasks delegated");
    let counter = IntCounterVec::new(opts, &["agent"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

/// Total worker restarts — labels: `agent`.
pub static WORKER_RESTARTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("worker_restarts_total", "Total worker restarts");
    let counter = IntCounterVec::new(opts, &["agent"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

/// Number of currently active agents.
pub static ACTIVE_AGENTS: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new("active_agents", "Number of active agents").unwrap();
    REGISTRY.register(Box::new(gauge.clone())).unwrap();
    gauge
});

/// Total tool executions — labels: `tool_name`, `status` (`success` | `error`).
pub static TOOL_EXECUTIONS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("tool_executions_total", "Total tool executions");
    let counter = IntCounterVec::new(opts, &["tool_name", "status"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

/// Run trace checkpoints — labels: `phase`, `status` (`ok` | `error` | `waiting` | `info`).
pub static RUN_TRACE_EVENTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("run_trace_events_total", "Structured run_trace checkpoints");
    let counter = IntCounterVec::new(opts, &["phase", "status"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

// ---------------------------------------------------------------------------
// Convenience recording functions
// ---------------------------------------------------------------------------

/// Record a completed LLM request.
pub fn record_llm_request(
    model: &str,
    success: bool,
    duration_secs: f64,
    prompt_tokens: u32,
    completion_tokens: u32,
) {
    let status = if success { "success" } else { "error" };
    LLM_REQUESTS_TOTAL.with_label_values(&[model, status]).inc();
    LLM_REQUEST_DURATION
        .with_label_values(&[model])
        .observe(duration_secs);
    LLM_TOKENS_TOTAL
        .with_label_values(&[model, "prompt"])
        .inc_by(prompt_tokens as u64);
    LLM_TOKENS_TOTAL
        .with_label_values(&[model, "completion"])
        .inc_by(completion_tokens as u64);
}

/// Record a task delegation event.
pub fn record_task_delegation(agent: &str) {
    TASK_DELEGATED_TOTAL.with_label_values(&[agent]).inc();
}

/// Record a tool execution event.
pub fn record_tool_execution(tool_name: &str, success: bool) {
    let status = if success { "success" } else { "error" };
    TOOL_EXECUTIONS_TOTAL
        .with_label_values(&[tool_name, status])
        .inc();
}

/// Record a [`crate::run_trace::RunTracer`] checkpoint.
pub fn record_run_trace(phase: &str, status: &str) {
    RUN_TRACE_EVENTS_TOTAL
        .with_label_values(&[phase, status])
        .inc();
}

// ---------------------------------------------------------------------------
// HTTP handler
// ---------------------------------------------------------------------------

/// GET /metrics — returns all registered metrics in Prometheus text format.
///
/// This endpoint intentionally requires no authentication so that Prometheus
/// scrapers can reach it without credentials.
pub async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_default()
}
