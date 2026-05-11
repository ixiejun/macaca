//! Framework-side Plugin Hook Bus integration helpers.
//!
//! The framework should not depend on `macaca-runtime-host` internals.  These
//! helpers construct bounded, protocol-level hook invocations that a shell or
//! service facade can send through the SDK Hook Bus client.  They intentionally
//! carry metadata, counts, identifiers, and sizes instead of raw prompts,
//! credentials, tool payloads, memory bodies, or provider-specific requests.

use std::collections::BTreeMap;

use macaca_proto::{
    PluginHookContext, PluginHookInvokeCommand, PluginHookKind, PluginHookName, ServiceScope,
    TraceContext,
};

/// Builder for safe framework hook invocations.
#[derive(Debug, Clone)]
pub struct FrameworkHookInvocationBuilder {
    hook_name: PluginHookName,
    expected_kind: Option<PluginHookKind>,
    context: PluginHookContext,
    payload: serde_json::Value,
    metadata: BTreeMap<String, String>,
}

impl FrameworkHookInvocationBuilder {
    /// Start a builder for one typed Hook Bus hook point.
    pub fn new(hook_name: PluginHookName) -> Self {
        Self {
            hook_name,
            expected_kind: None,
            context: PluginHookContext::global(),
            payload: serde_json::json!({}),
            metadata: BTreeMap::new(),
        }
    }

    /// Constrain invocation to one expected hook kind.
    pub fn expected_kind(mut self, kind: PluginHookKind) -> Self {
        self.expected_kind = Some(kind);
        self
    }

    /// Attach bounded lifecycle context.
    pub fn context(mut self, context: PluginHookContext) -> Self {
        self.context = context;
        self
    }

    /// Attach a bounded JSON payload.
    ///
    /// Callers must pass sanitized metadata only.  This method does not inspect
    /// provider secrets because the framework should never receive them here.
    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    /// Attach one audit-safe metadata field.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the final protocol command.
    pub fn build(self, trace: TraceContext) -> PluginHookInvokeCommand {
        PluginHookInvokeCommand {
            hook_name: self.hook_name,
            expected_kind: self.expected_kind,
            context: self.context,
            payload: self.payload,
            trace,
            metadata: self.metadata,
        }
    }
}

/// Build a bounded context for framework/session/task lifecycle hooks.
pub fn framework_hook_context(
    application_id: Option<String>,
    session_id: Option<String>,
    agent_name: Option<String>,
    task_id: Option<String>,
) -> PluginHookContext {
    let scope = session_id
        .clone()
        .map(ServiceScope::Session)
        .or_else(|| application_id.clone().map(ServiceScope::Application))
        .unwrap_or(ServiceScope::Global);
    PluginHookContext {
        application_id,
        session_id,
        agent_name,
        task_id,
        scope,
        metadata: BTreeMap::new(),
    }
}

/// Build a safe payload for prompt/context hooks.
pub fn prompt_context_payload(message_count: usize, prompt_chars: usize) -> serde_json::Value {
    serde_json::json!({
        "message_count": message_count,
        "prompt_chars": prompt_chars
    })
}

/// Build a safe payload for tool-call hooks.
pub fn tool_call_payload(tool_name: impl Into<String>, argument_bytes: usize) -> serde_json::Value {
    serde_json::json!({
        "tool_name": tool_name.into(),
        "argument_bytes": argument_bytes
    })
}

/// Build a safe payload for LLM hooks.
pub fn llm_call_payload(
    model_family: impl Into<String>,
    message_count: usize,
) -> serde_json::Value {
    serde_json::json!({
        "model_family": model_family.into(),
        "message_count": message_count
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_bounded_tool_hook_invocation() {
        let command = FrameworkHookInvocationBuilder::new(PluginHookName::BeforeToolCall)
            .expected_kind(PluginHookKind::Blocking)
            .context(framework_hook_context(
                Some("app".into()),
                Some("session".into()),
                Some("agent".into()),
                Some("task".into()),
            ))
            .payload(tool_call_payload("search", 128))
            .metadata("source", "framework")
            .build(TraceContext::new("trace-framework-hook"));

        assert_eq!(command.hook_name, PluginHookName::BeforeToolCall);
        assert_eq!(command.expected_kind, Some(PluginHookKind::Blocking));
        assert_eq!(command.payload["argument_bytes"], 128);
        assert_eq!(command.metadata.get("source").unwrap(), "framework");
    }
}
