//! Structured observability helpers for the application execution service.
//!
//! These helpers keep service logs uniform and bounded.  They deliberately
//! accept only identifiers, enums, status labels, and reason codes, so callers
//! cannot accidentally log raw prompts, provider payloads, manifests, WASM
//! bytes, credentials, or unbounded application data.

use std::collections::BTreeMap;

use macaca_proto::ApplicationExecutionProviderKind;

/// Bounded log record used by all application execution service nodes.
///
/// The record is a small Builder.  Callers can only attach protocol
/// identifiers, status labels, and reason codes; there is no field for raw
/// prompts, manifests, provider payloads, credentials, WASM bytes, or unbounded
/// application output.  Keeping one record shape gives operators a stable
/// audit surface while allowing new provider Strategies to add nodes without
/// changing the logging contract.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ApplicationExecutionLogRecord {
    node: String,
    application_id: Option<String>,
    session_id: Option<String>,
    run_id: Option<String>,
    provider_id: Option<String>,
    provider_kind: Option<ApplicationExecutionProviderKind>,
    command: Option<String>,
    trace_id: Option<String>,
    status: Option<String>,
    reason: Option<String>,
}

impl ApplicationExecutionLogRecord {
    /// Start a new record for one stable execution node.
    pub(crate) fn new(node: impl Into<String>) -> Self {
        Self {
            node: node.into(),
            ..Self::default()
        }
    }

    /// Attach the application identity as a protocol id, not a business name.
    pub(crate) fn application_id(mut self, application_id: impl Into<String>) -> Self {
        self.application_id = Some(application_id.into());
        self
    }

    /// Attach the session identity that scopes replay and diagnostics.
    pub(crate) fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Attach the run identity that scopes control delivery and event replay.
    pub(crate) fn run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    /// Attach the selected provider protocol id.
    pub(crate) fn provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    /// Attach the selected provider kind enum.
    pub(crate) fn provider_kind(mut self, provider_kind: ApplicationExecutionProviderKind) -> Self {
        self.provider_kind = Some(provider_kind);
        self
    }

    /// Attach the service command name that entered the service boundary.
    pub(crate) fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Attach the trace id that links this node to runtime audit evidence.
    pub(crate) fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Attach a stable bounded status label.
    pub(crate) fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Attach a stable bounded reason code when a branch rejects or fails.
    pub(crate) fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Return the exact sanitized fields that may be emitted to tracing.
    pub(crate) fn sanitized_fields(&self) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::from([("node".into(), self.node.clone())]);
        insert_if_some(
            &mut fields,
            "application_id",
            self.application_id.as_deref(),
        );
        insert_if_some(&mut fields, "session_id", self.session_id.as_deref());
        insert_if_some(&mut fields, "run_id", self.run_id.as_deref());
        insert_if_some(&mut fields, "provider_id", self.provider_id.as_deref());
        if let Some(provider_kind) = self.provider_kind {
            fields.insert("provider_kind".into(), format!("{provider_kind:?}"));
        }
        insert_if_some(&mut fields, "command", self.command.as_deref());
        insert_if_some(&mut fields, "trace_id", self.trace_id.as_deref());
        insert_if_some(&mut fields, "status", self.status.as_deref());
        insert_if_some(&mut fields, "reason", self.reason.as_deref());
        fields
    }

    /// Emit this record with a stable tracing schema.
    pub(crate) fn emit(&self, message: &'static str) {
        let fields = self.sanitized_fields();
        tracing::info!(
            node = %fields.get("node").map(String::as_str).unwrap_or("unknown"),
            application_id = fields.get("application_id").map(String::as_str),
            session_id = fields.get("session_id").map(String::as_str),
            run_id = fields.get("run_id").map(String::as_str),
            provider_id = fields.get("provider_id").map(String::as_str),
            provider_kind = fields.get("provider_kind").map(String::as_str),
            command = fields.get("command").map(String::as_str),
            trace_id = fields.get("trace_id").map(String::as_str),
            status = fields.get("status").map(String::as_str),
            reason = fields.get("reason").map(String::as_str),
            message
        );
    }
}

fn insert_if_some(fields: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        fields.insert(key.into(), value.into());
    }
}

/// Log application execution service lifecycle registration or startup.
pub(crate) fn log_service_lifecycle(
    service_id: &str,
    trace_id: Option<&str>,
    status: &str,
    configured: bool,
    reason: Option<&str>,
) {
    let node = if status == "registering" {
        "service_registration"
    } else {
        "service_start"
    };
    ApplicationExecutionLogRecord::new(node)
        .provider_id(service_id)
        .trace_id(trace_id.unwrap_or(""))
        .status(if configured { status } else { "unavailable" })
        .reason(reason.unwrap_or("none"))
        .emit("application execution service lifecycle");
}

/// Log command ingress before dispatching typed service behavior.
pub(crate) fn log_command_received(
    service_id: &str,
    command: &str,
    trace_id: &str,
    status: &str,
    reason: Option<&str>,
) {
    ApplicationExecutionLogRecord::new("command_receive")
        .provider_id(service_id)
        .command(command)
        .trace_id(trace_id)
        .status(status)
        .reason(reason.unwrap_or("none"))
        .emit("application execution command received");
}

/// Log provider assignment using only protocol identifiers.
pub(crate) fn log_provider_assignment(
    application_id: &str,
    session_id: &str,
    run_id: &str,
    provider_id: &str,
    provider_kind: ApplicationExecutionProviderKind,
    trace_id: &str,
) {
    ApplicationExecutionLogRecord::new("provider_assignment")
        .application_id(application_id)
        .session_id(session_id)
        .run_id(run_id)
        .provider_id(provider_id)
        .provider_kind(provider_kind)
        .trace_id(trace_id)
        .status("assigned")
        .emit("application execution provider assignment");
}

/// Log gateway ingress without exposing callback secrets or payload bodies.
pub(crate) fn log_gateway_ingress(
    application_id: &str,
    session_id: &str,
    run_id: &str,
    command: &str,
    trace_id: &str,
    status: &str,
) {
    ApplicationExecutionLogRecord::new("gateway_ingress")
        .application_id(application_id)
        .session_id(session_id)
        .run_id(run_id)
        .command(command)
        .trace_id(trace_id)
        .status(status)
        .emit("application execution gateway ingress");
}

/// Log a persisted protocol event after EventLog accepts the append.
pub(crate) fn log_event_append(
    application_id: &str,
    session_id: &str,
    run_id: &str,
    provider_id: &str,
    provider_kind: ApplicationExecutionProviderKind,
    event_type: &str,
    trace_id: &str,
    seq: u64,
) {
    ApplicationExecutionLogRecord::new("event_append")
        .application_id(application_id)
        .session_id(session_id)
        .run_id(run_id)
        .provider_id(provider_id)
        .provider_kind(provider_kind)
        .command(event_type)
        .trace_id(trace_id)
        .status(format!("appended:{seq}"))
        .emit("application execution event appended");
}

/// Log control routing after the request is durably recorded.
pub(crate) fn log_control_route(
    application_id: &str,
    session_id: &str,
    run_id: &str,
    provider_id: &str,
    provider_kind: ApplicationExecutionProviderKind,
    trace_id: &str,
    status: &str,
) {
    ApplicationExecutionLogRecord::new("control_route")
        .application_id(application_id)
        .session_id(session_id)
        .run_id(run_id)
        .provider_id(provider_id)
        .provider_kind(provider_kind)
        .command("application_execution.control")
        .trace_id(trace_id)
        .status(status)
        .emit("application execution control routed");
}

/// Log a snapshot request or snapshot failure using the same bounded schema.
pub(crate) fn log_snapshot(command: &str, trace_id: &str, status: &str, reason: Option<&str>) {
    ApplicationExecutionLogRecord::new("snapshot")
        .command(command)
        .trace_id(trace_id)
        .status(status)
        .reason(reason.unwrap_or("none"))
        .emit("application execution snapshot observed");
}

/// Log a service failure node without serializing raw error payloads.
pub(crate) fn log_failure(command: &str, trace_id: &str, reason: &str) {
    ApplicationExecutionLogRecord::new("failure")
        .command(command)
        .trace_id(trace_id)
        .status("failed")
        .reason(reason)
        .emit("application execution service failure");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_log_records_cover_required_nodes_with_bounded_fields_only() {
        let nodes = [
            "service_registration",
            "service_start",
            "command_receive",
            "policy_reject",
            "provider_assignment",
            "event_append",
            "control_route",
            "gateway_ingress",
            "snapshot",
            "failure",
        ];

        for node in nodes {
            let record = ApplicationExecutionLogRecord::new(node)
                .application_id("app-log-test")
                .session_id("session-log-test")
                .run_id("run-log-test")
                .provider_id("provider-log-test")
                .provider_kind(ApplicationExecutionProviderKind::MacacaHosted)
                .command("application_execution.test")
                .trace_id("trace-log-test")
                .status("observed")
                .reason("reason_code");
            let fields = record.sanitized_fields();

            assert_eq!(fields.get("node").map(String::as_str), Some(node));
            assert_eq!(
                fields.get("application_id").map(String::as_str),
                Some("app-log-test")
            );
            assert_eq!(
                fields.get("session_id").map(String::as_str),
                Some("session-log-test")
            );
            assert_eq!(
                fields.get("run_id").map(String::as_str),
                Some("run-log-test")
            );
            assert_eq!(
                fields.get("provider_id").map(String::as_str),
                Some("provider-log-test")
            );
            assert_eq!(
                fields.get("provider_kind").map(String::as_str),
                Some("MacacaHosted")
            );
            assert_eq!(
                fields.get("command").map(String::as_str),
                Some("application_execution.test")
            );
            assert_eq!(
                fields.get("trace_id").map(String::as_str),
                Some("trace-log-test")
            );
            assert_eq!(fields.get("status").map(String::as_str), Some("observed"));
            assert_eq!(
                fields.get("reason").map(String::as_str),
                Some("reason_code")
            );
            assert!(
                !fields.contains_key("payload")
                    && !fields.contains_key("prompt")
                    && !fields.contains_key("manifest")
                    && !fields.contains_key("secret")
            );
        }
    }
}
