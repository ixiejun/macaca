//! Driver command primitives.

use macaca_tools::TraceEvent;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

/// A typed command describing a driver tool execution request.
pub enum DriverCommand {
    Execute {
        tool_name: String,
        input: Value,
    },
    ExecuteStreaming {
        tool_name: String,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    },
}

impl DriverCommand {
    pub fn execute(tool_name: impl Into<String>, input: Value) -> Self {
        Self::Execute {
            tool_name: tool_name.into(),
            input,
        }
    }

    pub fn execute_streaming(
        tool_name: impl Into<String>,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    ) -> Self {
        Self::ExecuteStreaming {
            tool_name: tool_name.into(),
            input,
            event_tx,
        }
    }

    pub fn tool_name(&self) -> &str {
        match self {
            Self::Execute { tool_name, .. } | Self::ExecuteStreaming { tool_name, .. } => tool_name,
        }
    }

    pub fn input(&self) -> &Value {
        match self {
            Self::Execute { input, .. } | Self::ExecuteStreaming { input, .. } => input,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_exposes_tool_name_and_input() {
        let input = serde_json::json!({ "path": "README.md" });
        let command = DriverCommand::execute("file_read", input.clone());

        assert_eq!(command.tool_name(), "file_read");
        assert_eq!(command.input(), &input);
    }
}
