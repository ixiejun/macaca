//! Gateway manager — registers and coordinates message adapters.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info};

use macaca_proto::error::MacacaResult;
use macaca_proto::types::GatewayEvent;

use crate::adapter::{EventHandler, ImAdapter};

/// Central gateway that manages multiple IM adapters.
///
/// The gateway holds a list of registered adapters and an event handler.
/// When started, each adapter receives a reference to the handler so it
/// can dispatch incoming messages.
pub struct Gateway {
    adapters: Vec<Box<dyn ImAdapter>>,
    handler: Arc<dyn EventHandler>,
}

impl Gateway {
    /// Create a new gateway with the given event handler.
    pub fn new(handler: Arc<dyn EventHandler>) -> Self {
        Self {
            adapters: Vec::new(),
            handler,
        }
    }

    /// Register a message adapter with the gateway.
    pub fn register_adapter(&mut self, adapter: Box<dyn ImAdapter>) {
        info!(adapter = adapter.name(), "Registered gateway adapter");
        self.adapters.push(adapter);
    }

    /// Number of registered adapters.
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }

    /// Start all registered adapters, isolating per-adapter failures.
    ///
    /// Fault isolation (2026-07-08 audit): the previous implementation used `?`,
    /// so a single adapter that failed to start (e.g. an unavailable provider
    /// returning a structured error) aborted startup of every remaining adapter.
    /// We now start each adapter independently, record failures at `error`
    /// level, and continue, so one unavailable transport cannot take the whole
    /// gateway down. The count of failures is logged for observability.
    pub async fn start_all(&self) -> MacacaResult<()> {
        let mut failures = 0usize;
        for adapter in &self.adapters {
            info!(adapter = adapter.name(), "Starting adapter");
            if let Err(error) = adapter.start(Arc::clone(&self.handler)).await {
                failures += 1;
                error!(
                    adapter = adapter.name(),
                    error = %error,
                    "gateway adapter failed to start; continuing with remaining adapters"
                );
            }
        }
        info!(
            count = self.adapters.len(),
            failures, "gateway adapter startup complete"
        );
        Ok(())
    }

    /// Stop all registered adapters, isolating per-adapter failures.
    ///
    /// Mirrors [`Self::start_all`]: a failure stopping one adapter must not
    /// prevent the others from being stopped.
    pub async fn stop_all(&self) -> MacacaResult<()> {
        let mut failures = 0usize;
        for adapter in &self.adapters {
            info!(adapter = adapter.name(), "Stopping adapter");
            if let Err(error) = adapter.stop().await {
                failures += 1;
                error!(
                    adapter = adapter.name(),
                    error = %error,
                    "gateway adapter failed to stop; continuing with remaining adapters"
                );
            }
        }
        info!(failures, "gateway adapters stopped");
        Ok(())
    }
}

/// Default event handler that logs incoming events.
///
/// This is a stub implementation used during development. In production,
/// this would forward events to the kernel for task creation and routing.
pub struct DefaultEventHandler;

#[async_trait]
impl EventHandler for DefaultEventHandler {
    async fn handle(&self, event: GatewayEvent) -> MacacaResult<()> {
        match &event {
            GatewayEvent::TaskRequest {
                user_id,
                channel_id,
                content,
            } => {
                // Log a bounded preview, not the full message (2026-07-08 audit
                // S9): user content can be large and sensitive.
                info!(
                    user_id = %user_id,
                    channel_id = %channel_id,
                    content_preview = %macaca_proto::text_sanitize::truncate_with_marker(content, 128),
                    "Received task request"
                );
            }
            GatewayEvent::StatusQuery {
                user_id,
                channel_id,
                task_id,
            } => {
                info!(
                    user_id = %user_id,
                    channel_id = %channel_id,
                    task_id = ?task_id,
                    "Received status query"
                );
            }
            GatewayEvent::UserReply {
                user_id,
                channel_id,
                content,
                context_id,
            } => {
                // Bounded preview only (2026-07-08 audit S9).
                info!(
                    user_id = %user_id,
                    channel_id = %channel_id,
                    content_preview = %macaca_proto::text_sanitize::truncate_with_marker(content, 128),
                    context_id = %context_id,
                    "Received user reply"
                );
            }
            GatewayEvent::Command {
                user_id,
                channel_id,
                command,
                args,
            } => {
                info!(
                    user_id = %user_id,
                    channel_id = %channel_id,
                    command = %command,
                    args = ?args,
                    "Received command"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockAdapter {
        name: String,
        started: AtomicUsize,
        stopped: AtomicUsize,
    }

    impl MockAdapter {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                started: AtomicUsize::new(0),
                stopped: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl ImAdapter for MockAdapter {
        fn name(&self) -> &str {
            &self.name
        }

        async fn start(&self, _handler: Arc<dyn EventHandler>) -> MacacaResult<()> {
            self.started.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn send_message(&self, _channel_id: &str, _content: &str) -> MacacaResult<()> {
            Ok(())
        }

        async fn stop(&self) -> MacacaResult<()> {
            self.stopped.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct CountingHandler {
        count: AtomicUsize,
    }

    impl CountingHandler {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
            }
        }

        fn event_count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl EventHandler for CountingHandler {
        async fn handle(&self, _event: GatewayEvent) -> MacacaResult<()> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn gateway_register_adapters() {
        let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
        let mut gw = Gateway::new(handler);
        assert_eq!(gw.adapter_count(), 0);

        gw.register_adapter(Box::new(MockAdapter::new("test1")));
        assert_eq!(gw.adapter_count(), 1);

        gw.register_adapter(Box::new(MockAdapter::new("test2")));
        assert_eq!(gw.adapter_count(), 2);
    }

    #[tokio::test]
    async fn gateway_start_all() {
        let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
        let mut gw = Gateway::new(handler);

        gw.register_adapter(Box::new(MockAdapter::new("a1")));
        gw.register_adapter(Box::new(MockAdapter::new("a2")));

        gw.start_all().await.unwrap();
        assert_eq!(gw.adapter_count(), 2);
    }

    #[tokio::test]
    async fn gateway_stop_all() {
        let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
        let mut gw = Gateway::new(handler);
        gw.register_adapter(Box::new(MockAdapter::new("adapter")));

        gw.start_all().await.unwrap();
        gw.stop_all().await.unwrap();
    }

    #[tokio::test]
    async fn gateway_empty_start_stop() {
        let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
        let gw = Gateway::new(handler);
        gw.start_all().await.unwrap();
        gw.stop_all().await.unwrap();
    }

    #[tokio::test]
    async fn default_event_handler_handles_task_request() {
        let handler = DefaultEventHandler;
        let event = GatewayEvent::TaskRequest {
            user_id: "u1".into(),
            channel_id: "c1".into(),
            content: "build an app".into(),
        };
        handler.handle(event).await.unwrap();
    }

    #[tokio::test]
    async fn default_event_handler_handles_status_query() {
        let handler = DefaultEventHandler;
        let event = GatewayEvent::StatusQuery {
            user_id: "u1".into(),
            channel_id: "c1".into(),
            task_id: None,
        };
        handler.handle(event).await.unwrap();
    }

    #[tokio::test]
    async fn default_event_handler_handles_command() {
        let handler = DefaultEventHandler;
        let event = GatewayEvent::Command {
            user_id: "u1".into(),
            channel_id: "c1".into(),
            command: "help".into(),
            args: vec!["--verbose".into()],
        };
        handler.handle(event).await.unwrap();
    }

    #[tokio::test]
    async fn counting_handler_tracks_events() {
        let handler = Arc::new(CountingHandler::new());
        assert_eq!(handler.event_count(), 0);

        let event = GatewayEvent::TaskRequest {
            user_id: "u1".into(),
            channel_id: "c1".into(),
            content: "test".into(),
        };
        handler.handle(event).await.unwrap();
        assert_eq!(handler.event_count(), 1);

        let event2 = GatewayEvent::Command {
            user_id: "u2".into(),
            channel_id: "c2".into(),
            command: "status".into(),
            args: vec![],
        };
        handler.handle(event2).await.unwrap();
        assert_eq!(handler.event_count(), 2);
    }
}
