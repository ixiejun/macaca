//! SSE stream adapter for chat routes.
//!
//! Normalizes the first `session_id` event and keep-alive wrapper so framework
//! and WASM branches return the same Axum `Sse` concrete type.

use std::convert::Infallible;

use axum::response::sse::{Event, KeepAlive, Sse};

/// Build the normalized SSE response shape used by all chat execution paths.
///
/// Keeping one constructor avoids divergent stream concrete types between
/// framework and WASM fast-path branches and guarantees a stable first
/// `session_id` event for the frontend.
pub(crate) fn build_chat_sse(
    session_key: String,
    stream_rx: tokio::sync::mpsc::Receiver<Result<Event, Infallible>>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut stream_rx = stream_rx;
        yield Ok(Event::default()
            .event("session_id")
            .data(serde_json::json!({"session_id": session_key}).to_string()));
        while let Some(event) = stream_rx.recv().await {
            yield event;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}
