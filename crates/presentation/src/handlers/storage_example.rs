// Example: stream a stored file as an HTTP response.
// Remove this file or replace with your own handlers.
//
// To use, add to your router:
//   .route("/files/*key", get(storage_example::get_file))
//
// use axum::{
//     body::Body,
//     extract::{Path, State},
//     http::StatusCode,
//     response::IntoResponse,
// };
// use futures::StreamExt;
// use crate::state::AppState;
//
// pub async fn get_file(
//     Path(key): Path<String>,
//     State(state): State<AppState>,
// ) -> Result<impl IntoResponse, StatusCode> {
//     let stream = state
//         .storage
//         .get(&key)
//         .await
//         .map_err(|_| StatusCode::NOT_FOUND)?;
//     let body = Body::from_stream(stream.map(|r| r.map_err(|e| e.to_string())));
//     Ok(body)
// }
