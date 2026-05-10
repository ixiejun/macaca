//! Observer-style event emission for runtime execution.

use macaca_proto::AgentExecutionEvent;
use tokio::sync::mpsc;

/// Small wrapper around an optional runtime event channel.
pub(crate) struct RuntimeEventSink<'a> {
    tx: Option<&'a mpsc::Sender<AgentExecutionEvent>>,
}

impl<'a> RuntimeEventSink<'a> {
    pub(crate) fn new(tx: Option<&'a mpsc::Sender<AgentExecutionEvent>>) -> Self {
        Self { tx }
    }

    pub(crate) async fn emit(&self, event: AgentExecutionEvent) {
        if let Some(tx) = self.tx {
            let _ = tx.send(event).await;
        }
    }

    pub(crate) async fn emit_thinking(&self, iteration: usize) {
        self.emit(AgentExecutionEvent::thinking(iteration)).await;
    }

    pub(crate) async fn emit_assistant(&self, content: String) {
        if !content.is_empty() {
            self.emit(AgentExecutionEvent::assistant(content)).await;
        }
    }

    pub(crate) async fn emit_completed(&self, success: bool, error: Option<String>) {
        self.emit(AgentExecutionEvent::completed(success, error))
            .await;
    }

    pub(crate) fn sender(&self) -> Option<&'a mpsc::Sender<AgentExecutionEvent>> {
        self.tx
    }
}

impl<'a> From<Option<&'a mpsc::Sender<AgentExecutionEvent>>> for RuntimeEventSink<'a> {
    fn from(tx: Option<&'a mpsc::Sender<AgentExecutionEvent>>) -> Self {
        Self::new(tx)
    }
}
