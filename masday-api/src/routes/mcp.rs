//! MCP HTTP transport routes (SSE + Streamable HTTP)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

/// Query params for SSE message endpoint
#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub session_id: Option<String>,
}

/// GET /mcp/sse — SSE transport endpoint
///
/// Opens an SSE stream for server-to-client communication.
/// Sends initial `endpoint` event with the POST URL for client-to-server messages.
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (session_id, rx) = state.mcp_sessions.create_session();

    // Build the endpoint URL for this session
    let endpoint = format!("/mcp/messages?session_id={}", session_id);

    // Create initial endpoint event
    let endpoint_event = Event::default().event("endpoint").data(endpoint);

    // Convert broadcast receiver to stream
    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        match msg {
            Ok(data) => Some(Ok(Event::default().event("message").data(data))),
            Err(_) => None, // Skip lagged messages
        }
    });

    // Prepend the endpoint event
    let combined = futures::stream::once(async move { Ok(endpoint_event) }).chain(stream);

    Sse::new(combined).keep_alive(KeepAlive::default())
}

/// POST /mcp/messages — SSE transport message endpoint
///
/// Accepts JSON-RPC request body, dispatches to McpHandler,
/// sends response via SSE broadcast channel.
pub async fn messages_handler(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
    body: String,
) -> impl IntoResponse {
    let session_id = match query.session_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing session_id parameter"})),
            )
                .into_response();
        }
    };

    // Dispatch to MCP handler
    let response = state.mcp_handler.handle_raw_json(&body).await;

    // Send response via SSE channel if there is one
    if let Some(json_response) = response {
        state
            .mcp_sessions
            .send_to_session(&session_id, &json_response);
    }

    StatusCode::ACCEPTED.into_response()
}

/// POST /mcp — Streamable HTTP transport endpoint
///
/// Direct JSON-RPC request/response over HTTP.
pub async fn streamable_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    let json_str = serde_json::to_string(&request).unwrap_or_default();
    let response = state.mcp_handler.handle_raw_json(&json_str).await;

    match response {
        Some(json_response) => {
            let parsed: serde_json::Value =
                serde_json::from_str(&json_response).unwrap_or(serde_json::json!({}));
            (StatusCode::OK, Json(parsed)).into_response()
        }
        None => StatusCode::NO_CONTENT.into_response(),
    }
}
