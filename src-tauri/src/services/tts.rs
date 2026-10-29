//! VieNeu-TTS through its OpenAI-compatible server (`python -m apps.openai_speech`, see scripts/start-tts.*).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::pipeline::render::SAMPLE_RATE;

const SERVICE: &str = "VieNeu-TTS";
/// CPU synthesis runs at roughly 0.6x real time; a long narration still finishes well inside this.
const SPEAK_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Voice {
    pub id: String,
    /// e.g. "Nam · Bắc · Phong cách kể chuyện".
    #[serde(default)]
    pub description: String,
}

pub struct Tts {
    http: reqwest::Client,
    base: String,
}

impl Tts {
    pub fn new(http: reqwest::Client, base: &str) -> Self {
        Self { http, base: base.trim().trim_end_matches('/').to_owned() }
    }

    pub async fn voices(&self) -> AppResult<Vec<Voice>> {
        let res =
            self.http.get(format!("{}/v1/voices", self.base)).timeout(Duration::from_secs(5)).send().await.map_err(|e| self.err(&e))?;
        let body: Value = check(res).await?.json().await.map_err(|e| self.err(&e))?;
        Ok(serde_json::from_value(body["data"].clone())?)
    }

    /// Mono 16-bit samples at `SAMPLE_RATE`. An empty `voice` uses the server's default.
    pub async fn speak(&self, text: &str, voice: &str) -> AppResult<Vec<u8>> {
        let body = json!({
            "input": text,
            "voice": (!voice.is_empty()).then_some(voice),
            "response_format": "pcm",
            "sample_rate": SAMPLE_RATE,
        });
        let res = self
            .http
            .post(format!("{}/v1/audio/speech", self.base))
            .timeout(SPEAK_TIMEOUT)
            .json(&body)
            .send()
            .await
            .map_err(|e| self.err(&e))?;
        Ok(check(res).await?.bytes().await.map_err(|e| self.err(&e))?.to_vec())
    }

    fn err(&self, e: &reqwest::Error) -> AppError {
        if e.is_connect() {
            AppError::service(
                SERVICE,
                format!(
                    "Không kết nối được {}. Chạy scripts/start-tts.ps1 (Mac: start-tts.sh) và chờ dòng \"ready\"; lần đầu phải tải model nên lâu hơn.",
                    self.base
                ),
            )
        } else {
            AppError::service(SERVICE, e.to_string())
        }
    }
}

/// The server answers errors as `{ "error": { "message": ... } }`, e.g. an unknown voice.
async fn check(res: reqwest::Response) -> AppResult<reqwest::Response> {
    if res.status().is_success() {
        return Ok(res);
    }
    let status = res.status();
    let body: Value = res.json().await.unwrap_or_default();
    let msg = body["error"]["message"].as_str().map_or_else(|| status.to_string(), str::to_owned);
    Err(AppError::service(SERVICE, msg))
}
