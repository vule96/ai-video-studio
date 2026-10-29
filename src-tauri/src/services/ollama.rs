use std::time::Duration;

use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::cancellable;
use crate::error::{AppError, AppResult};

const SERVICE: &str = "Ollama";
/// Long PDFs and large models can take minutes per call on a mid-range GPU.
const CHAT_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// One request to the model.
pub struct Prompt<'a> {
    pub system: &'a str,
    pub user: &'a str,
    /// Base64 images for vision models.
    pub images: &'a [String],
    /// Structured output: the reply is JSON matching this schema.
    pub schema: Option<&'a Value>,
    /// Keep the model in VRAM after the call (a batch of calls); otherwise unload it so ComfyUI gets the GPU.
    pub keep_loaded: bool,
}

impl<'a> Prompt<'a> {
    pub fn new(system: &'a str, user: &'a str) -> Self {
        Self { system, user, images: &[], schema: None, keep_loaded: false }
    }
}

pub struct Ollama {
    http: reqwest::Client,
    base: String,
}

impl Ollama {
    pub fn new(http: reqwest::Client, base: &str) -> Self {
        Self { http, base: base.trim_end_matches('/').to_owned() }
    }

    pub async fn models(&self) -> AppResult<Vec<String>> {
        let res = self.http.get(format!("{}/api/tags", self.base)).timeout(Duration::from_secs(5)).send().await.map_err(unreachable)?;
        let body: Value = res.json().await.map_err(bad_response)?;
        Ok(body["models"].as_array().into_iter().flatten().filter_map(|m| m["name"].as_str().map(str::to_owned)).collect())
    }

    /// What the model can do, e.g. `["completion", "vision"]`. Empty on Ollama builds that predate the field.
    pub async fn capabilities(&self, model: &str) -> AppResult<Vec<String>> {
        let res = self
            .http
            .post(format!("{}/api/show", self.base))
            .timeout(Duration::from_secs(5))
            .json(&json!({ "model": model }))
            .send()
            .await
            .map_err(unreachable)?;
        let body: Value = res.json().await.map_err(bad_response)?;
        Ok(body["capabilities"].as_array().into_iter().flatten().filter_map(|c| c.as_str().map(str::to_owned)).collect())
    }

    /// Frees the model's VRAM after a batch that used `keep_loaded`. Best effort.
    pub async fn unload(&self, model: &str) {
        let body = json!({ "model": model, "keep_alive": 0 });
        let _ = self.http.post(format!("{}/api/generate", self.base)).timeout(Duration::from_secs(10)).json(&body).send().await;
    }

    /// One chat turn.
    pub async fn chat(&self, model: &str, p: &Prompt<'_>, cancel: &CancellationToken) -> AppResult<String> {
        let mut user = json!({ "role": "user", "content": p.user });
        if !p.images.is_empty() {
            user["images"] = json!(p.images);
        }
        let mut body = json!({
            "model": model,
            "messages": [{ "role": "system", "content": p.system }, user],
            "stream": false,
            "keep_alive": if p.keep_loaded { json!("5m") } else { json!(0) },
            "think": false,
            "options": { "temperature": 0.4, "num_ctx": 12288 },
        });
        if let Some(schema) = p.schema {
            body["format"] = schema.clone();
        }
        let out = match cancellable(cancel, self.post_chat(&body)).await {
            // Older Ollama builds reject the `think` flag for models without a thinking mode.
            Err(AppError::Service { message, .. }) if message.contains("think") => {
                if let Some(obj) = body.as_object_mut() {
                    obj.remove("think");
                }
                cancellable(cancel, self.post_chat(&body)).await?
            }
            other => other?,
        };
        let content = out["message"]["content"].as_str().unwrap_or_default().trim().to_owned();
        if content.is_empty() {
            return Err(AppError::service(SERVICE, "Model trả về nội dung rỗng"));
        }
        Ok(content)
    }

    /// Structured output with up to two retries. Models sometimes wrap JSON in prose; salvage the outer `{...}`.
    pub async fn chat_json(&self, model: &str, p: &Prompt<'_>, cancel: &CancellationToken) -> AppResult<Value> {
        let mut last = String::new();
        for _ in 0..3 {
            let raw = self.chat(model, p, cancel).await?;
            if let Some(v) = parse_json_object(&raw) {
                return Ok(v);
            }
            last = raw.chars().take(200).collect();
        }
        Err(AppError::service(SERVICE, format!("Model không trả về JSON hợp lệ sau 3 lần thử. Đoạn cuối: {last}")))
    }

    async fn post_chat(&self, body: &Value) -> AppResult<Value> {
        let res = self.http.post(format!("{}/api/chat", self.base)).timeout(CHAT_TIMEOUT).json(body).send().await.map_err(unreachable)?;
        let status = res.status();
        let body: Value = res.json().await.map_err(bad_response)?;
        if !status.is_success() {
            let msg = body["error"].as_str().unwrap_or("lỗi không rõ");
            let hint =
                if msg.contains("not found") { " — chạy `ollama pull <model>` hoặc đổi model trong Cài đặt" } else { "" };
            return Err(AppError::service(SERVICE, format!("{msg}{hint}")));
        }
        Ok(body)
    }
}

fn parse_json_object(raw: &str) -> Option<Value> {
    if let Ok(v @ Value::Object(_)) = serde_json::from_str(raw) {
        return Some(v);
    }
    let (start, end) = (raw.find('{')?, raw.rfind('}')?);
    match serde_json::from_str(raw.get(start..=end)?) {
        Ok(v @ Value::Object(_)) => Some(v),
        _ => None,
    }
}

fn unreachable(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::service(SERVICE, "Quá thời gian chờ phản hồi")
    } else {
        AppError::service(SERVICE, format!("Không kết nối được ({e}). Hãy mở Ollama."))
    }
}

fn bad_response(e: reqwest::Error) -> AppError {
    AppError::service(SERVICE, format!("Phản hồi không đọc được: {e}"))
}

#[cfg(test)]
mod tests {
    use super::parse_json_object;

    #[test]
    fn salvages_json_wrapped_in_prose() {
        assert!(parse_json_object(r#"{"a":1}"#).is_some());
        assert_eq!(parse_json_object("Here you go:\n```json\n{\"a\": {\"b\": 2}}\n```").unwrap()["a"]["b"], 2);
        assert!(parse_json_object("[1,2]").is_none());
        assert!(parse_json_object("no json").is_none());
        assert!(parse_json_object("} reversed {").is_none());
    }
}
