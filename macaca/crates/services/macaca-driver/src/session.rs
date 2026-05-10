//! Driver execution session state.

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::TraceEvent;
use tokio::sync::mpsc::UnboundedSender;

use crate::trace::DriverTraceAdapter;

/// State carried by a streaming driver execution callback.
pub struct DriverSessionState {
    tx: UnboundedSender<TraceEvent>,
    trace_adapter: DriverTraceAdapter,
}

impl DriverSessionState {
    pub fn new(tx: UnboundedSender<TraceEvent>, driver_name: impl Into<String>) -> Self {
        Self {
            tx,
            trace_adapter: DriverTraceAdapter::new(driver_name),
        }
    }

    pub fn forward_json_event(&self, json: &str) -> MacacaResult<()> {
        let event = serde_json::from_str::<TraceEvent>(json).map_err(|e| {
            MacacaError::Driver(format!("Invalid streaming trace event JSON: {}", e))
        })?;
        let _ = self.tx.send(self.trace_adapter.enrich(event));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn forward_json_event_enriches_and_sends_event() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let session = DriverSessionState::new(tx, "driver-a");

        session
            .forward_json_event(r#"{"type":"thinking","content":"working"}"#)
            .unwrap();

        let event = rx.recv().await.unwrap();
        assert_eq!(event.event_type, "thinking");
        assert_eq!(event.driver_id.as_deref(), Some("driver-a"));
        assert!(event.timestamp.is_some());
    }
}
