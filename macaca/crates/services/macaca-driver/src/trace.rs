//! Driver trace adapters.

use std::time::{SystemTime, UNIX_EPOCH};

use macaca_tools::TraceEvent;

/// Enriches raw driver trace events with Macaca-required metadata.
#[derive(Debug, Clone)]
pub struct DriverTraceAdapter {
    driver_name: String,
}

impl DriverTraceAdapter {
    pub fn new(driver_name: impl Into<String>) -> Self {
        Self {
            driver_name: driver_name.into(),
        }
    }

    pub fn enrich(&self, mut event: TraceEvent) -> TraceEvent {
        if event.driver_id.is_none() {
            event.driver_id = Some(self.driver_name.clone());
        }
        if event.timestamp.is_none() {
            event.timestamp = now_millis();
        }
        event
    }
}

fn now_millis() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrich_fills_missing_metadata() {
        let adapter = DriverTraceAdapter::new("driver-a");
        let event = TraceEvent {
            event_type: "tool_call".into(),
            ..TraceEvent::default()
        };

        let enriched = adapter.enrich(event);

        assert_eq!(enriched.driver_id.as_deref(), Some("driver-a"));
        assert!(enriched.timestamp.is_some());
    }

    #[test]
    fn enrich_preserves_existing_metadata() {
        let adapter = DriverTraceAdapter::new("driver-a");
        let event = TraceEvent {
            event_type: "tool_call".into(),
            driver_id: Some("driver-b".into()),
            timestamp: Some(42),
            ..TraceEvent::default()
        };

        let enriched = adapter.enrich(event);

        assert_eq!(enriched.driver_id.as_deref(), Some("driver-b"));
        assert_eq!(enriched.timestamp, Some(42));
    }
}
