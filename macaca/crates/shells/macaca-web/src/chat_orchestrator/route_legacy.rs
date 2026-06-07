//! Legacy POST /api/chat compatibility shim.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;

use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

use super::dto::ChatRequest;
use super::route_chat_v2::post_chat_v2;

/// Legacy compatibility shim.
///
/// `/api/chat` route has been removed from router registration, and framework
/// execution is now the only supported business path. If invoked directly,
/// forward to the framework handler.
#[deprecated(note = "Use post_chat_v2; legacy /api/chat is removed.")]
pub(crate) async fn post_chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    post_chat_v2(State(state), Json(req)).await
}
