use macaca_proto::{AgentExecutionEvent, AgentExecutionEventVisitor};

struct DelegatedEventNameVisitor {
    completed_name: &'static str,
}

impl AgentExecutionEventVisitor<&'static str> for DelegatedEventNameVisitor {
    fn thinking(&mut self, _iteration: usize, _content: Option<&str>) -> &'static str {
        "delegated_thinking"
    }

    fn tool_call(
        &mut self,
        _tool_name: &str,
        _tool_input: &serde_json::Value,
        _call_id: Option<&str>,
    ) -> &'static str {
        "delegated_tool_call"
    }

    fn tool_result(
        &mut self,
        _tool_name: &str,
        _output: &str,
        _is_error: Option<bool>,
    ) -> &'static str {
        "delegated_tool_result"
    }

    fn assistant(&mut self, _content: &str) -> &'static str {
        "delegated_assistant"
    }

    fn driver_trace(&mut self, _driver_name: &str, _trace: &serde_json::Value) -> &'static str {
        "delegated_driver_trace"
    }

    fn completed(&mut self, _success: bool, _error: Option<&str>) -> &'static str {
        self.completed_name
    }
}

pub(crate) fn delegated_sse_event_name(event: &AgentExecutionEvent) -> &'static str {
    let mut visitor = DelegatedEventNameVisitor {
        completed_name: "delegated_completed",
    };
    event.accept(&mut visitor)
}

pub(crate) fn delegated_persisted_event_name(event: &AgentExecutionEvent) -> &'static str {
    let mut visitor = DelegatedEventNameVisitor {
        completed_name: "delegated_done",
    };
    event.accept(&mut visitor)
}
