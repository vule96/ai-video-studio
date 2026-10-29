use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "ComfyUI";
/// Portable build listens on 8188, the Desktop app on 8000.
const DEFAULT_URLS: [&str; 2] = ["http://127.0.0.1:8188", "http://127.0.0.1:8000"];
const SHORT: Duration = Duration::from_secs(10);
/// Model loading on first use can take a few minutes.
const RUN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub struct Comfy {
    http: reqwest::Client,
    pub base: String,
}

impl Comfy {
    /// Uses `configured` if set, otherwise the first default port that answers.
    pub async fn connect(http: reqwest::Client, configured: &str) -> AppResult<Self> {
        let candidates: Vec<&str> = if configured.trim().is_empty() { DEFAULT_URLS.to_vec() } else { vec![configured.trim()] };
        for base in candidates {
            let base = base.trim_end_matches('/');
            let ok = http
                .get(format!("{base}/system_stats"))
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success());
            if ok {
                return Ok(Self { http, base: base.to_owned() });
            }
        }
        Err(AppError::service(
            SERVICE,
            "Không tìm thấy ComfyUI (đã thử cổng 8188 và 8000). Hãy mở ComfyUI hoặc nhập địa chỉ trong Cài đặt.",
        ))
    }

    /// Choices ComfyUI offers for a loader input, i.e. the model files it can see.
    pub async fn options(&self, node: &str, input: &str) -> AppResult<Vec<String>> {
        let info: Value = self.get_json(&format!("/object_info/{node}")).await?;
        let spec = &info[node]["input"]["required"][input];
        // Old schema: [[...options]]; new schema: ["COMBO", { "options": [...] }]
        let list = spec[0].as_array().or_else(|| spec[1]["options"].as_array());
        let Some(list) = list else {
            return Err(AppError::service(SERVICE, format!("ComfyUI không có node {node}. Hãy cập nhật ComfyUI.")));
        };
        Ok(list.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
    }

    /// Copies an image into ComfyUI's input folder; returns the name for `LoadImage`.
    pub async fn upload_image(&self, path: &Path, name: &str) -> AppResult<String> {
        let bytes = tokio::fs::read(path).await?;
        let part = reqwest::multipart::Part::bytes(bytes).file_name(name.to_owned()).mime_str("image/png").map_err(http_err)?;
        let form = reqwest::multipart::Form::new().part("image", part).text("overwrite", "true");
        let res = self.http.post(format!("{}/upload/image", self.base)).timeout(SHORT).multipart(form).send().await.map_err(http_err)?;
        let body: Value = res.error_for_status().map_err(http_err)?.json().await.map_err(http_err)?;
        let name = body["name"].as_str().ok_or_else(|| AppError::service(SERVICE, "Upload ảnh không trả về tên file"))?;
        Ok(match body["subfolder"].as_str() {
            Some(sub) if !sub.is_empty() => format!("{sub}/{name}"),
            _ => name.to_owned(),
        })
    }

    /// Queues a workflow and returns the first image it saved.
    pub async fn run(&self, graph: &Value, cancel: &CancellationToken) -> AppResult<Vec<u8>> {
        let res = self
            .http
            .post(format!("{}/prompt", self.base))
            .timeout(SHORT)
            .json(&json!({ "prompt": graph }))
            .send()
            .await
            .map_err(http_err)?;
        let status = res.status();
        let body: Value = res.json().await.map_err(http_err)?;
        if !status.is_success() {
            return Err(AppError::service(SERVICE, explain_validation_error(&body)));
        }
        let id = body["prompt_id"].as_str().ok_or_else(|| AppError::service(SERVICE, "Không nhận được prompt_id"))?.to_owned();

        let started = tokio::time::Instant::now();
        // shortcut: polls /history once a second; switch to the /ws progress socket when the UI needs per-step progress.
        let entry = loop {
            if cancel.is_cancelled() {
                self.cancel_prompt(&id).await;
                return Err(AppError::Cancelled);
            }
            if started.elapsed() > RUN_TIMEOUT {
                self.cancel_prompt(&id).await;
                return Err(AppError::service(SERVICE, "Quá thời gian chờ sinh ảnh"));
            }
            // A prompt appears in history only once it has finished, successfully or not.
            let mut hist: Value = self.get_json(&format!("/history/{id}")).await?;
            let entry = hist[&id].take();
            if !entry.is_null() {
                break entry;
            }
            tokio::select! {
                () = cancel.cancelled() => {}
                () = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
        };

        if entry["status"]["status_str"] == "error" {
            return Err(AppError::service(SERVICE, format!("Lỗi khi chạy workflow: {}", execution_error(&entry))));
        }
        let image = entry["outputs"]
            .as_object()
            .into_iter()
            .flat_map(|o| o.values())
            .flat_map(|out| out["images"].as_array().into_iter().flatten())
            .find(|img| img["type"] == "output")
            .ok_or_else(|| AppError::service(SERVICE, "Workflow chạy xong nhưng không có ảnh đầu ra"))?;
        let query = [
            ("filename", image["filename"].as_str().unwrap_or_default()),
            ("subfolder", image["subfolder"].as_str().unwrap_or_default()),
            ("type", "output"),
        ];
        let res = self.http.get(format!("{}/view", self.base)).query(&query).timeout(RUN_TIMEOUT).send().await.map_err(http_err)?;
        Ok(res.error_for_status().map_err(http_err)?.bytes().await.map_err(http_err)?.to_vec())
    }

    /// Unloads models so Ollama can use the GPU. Best effort: a failure here must not fail the job.
    pub async fn free(&self) {
        let _ = self
            .http
            .post(format!("{}/free", self.base))
            .timeout(SHORT)
            .json(&json!({ "unload_models": true, "free_memory": true }))
            .send()
            .await;
    }

    async fn cancel_prompt(&self, id: &str) {
        let _ = self.http.post(format!("{}/queue", self.base)).timeout(SHORT).json(&json!({ "delete": [id] })).send().await;
        let _ = self.http.post(format!("{}/interrupt", self.base)).timeout(SHORT).json(&json!({ "prompt_id": id })).send().await;
    }

    async fn get_json(&self, path: &str) -> AppResult<Value> {
        let res = self.http.get(format!("{}{path}", self.base)).timeout(SHORT).send().await.map_err(http_err)?;
        res.error_for_status().map_err(http_err)?.json().await.map_err(http_err)
    }
}

fn http_err(e: reqwest::Error) -> AppError {
    AppError::service(SERVICE, e.to_string())
}

fn execution_error(entry: &Value) -> String {
    entry["status"]["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|m| m[0] == "execution_error")
        .and_then(|m| m[1]["exception_message"].as_str())
        .unwrap_or("không rõ")
        .trim()
        .to_owned()
}

/// ComfyUI's `/prompt` validation JSON → readable message with a hint for the usual causes.
fn explain_validation_error(body: &Value) -> String {
    let mut lines = vec![body["error"]["message"].as_str().unwrap_or("Workflow không hợp lệ").to_owned()];
    for (node, err) in body["node_errors"].as_object().into_iter().flatten() {
        for e in err["errors"].as_array().into_iter().flatten() {
            let class = err["class_type"].as_str().unwrap_or("?");
            let msg = e["message"].as_str().unwrap_or_default();
            let details = e["details"].as_str().unwrap_or_default();
            lines.push(format!("node {node} ({class}): {msg} {details}").trim_end().to_owned());
        }
    }
    let text = lines.join("\n");
    if text.contains("not in list") {
        format!("{text}\n→ ComfyUI chưa có file model này. Kiểm tra mục Model trong Cài đặt và thư mục ComfyUI/models.")
    } else if text.contains("does not exist") {
        format!("{text}\n→ ComfyUI quá cũ, chưa có node này. Hãy cập nhật ComfyUI.")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_error_gets_a_hint() {
        let body = json!({
            "error": { "message": "Prompt outputs failed validation" },
            "node_errors": { "1": { "class_type": "UNETLoader", "errors": [
                { "message": "Value not in list", "details": "unet_name: 'x.safetensors' not in []" }
            ]}}
        });
        let msg = explain_validation_error(&body);
        assert!(msg.contains("node 1 (UNETLoader)"));
        assert!(msg.contains("chưa có file model"));
    }
}
