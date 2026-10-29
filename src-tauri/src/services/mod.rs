//! Clients for the local services: Ollama, ComfyUI and VieNeu-TTS over HTTP, ffmpeg as a process.
//! Nothing is bundled into the app.

pub mod comfy;
pub mod ffmpeg;
pub mod ollama;
pub mod tts;

use std::future::Future;

use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};

/// Runs `fut` until it finishes or the job is cancelled. Dropping an HTTP future closes the
/// connection, which also stops generation on the Ollama side.
pub async fn cancellable<T>(cancel: &CancellationToken, fut: impl Future<Output = AppResult<T>>) -> AppResult<T> {
    tokio::select! {
        () = cancel.cancelled() => Err(AppError::Cancelled),
        out = fut => out,
    }
}
