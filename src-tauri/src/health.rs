//! "Is everything ready?" for the Settings screen: services reachable, model files present, ffmpeg installed.

use serde::Serialize;
use ts_rs::TS;

use crate::pipeline::workflow::{RIFE, UPSCALER, WAN_HIGH, WAN_LORA_HIGH, WAN_LORA_LOW, WAN_LOW, WAN_TEXT_ENCODER, WAN_VAE};
use crate::services::comfy::Comfy;
use crate::services::ffmpeg;
use crate::services::ollama::Ollama;
use crate::services::tts::{Tts, Voice};
use crate::settings::Settings;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Check {
    pub name: String,
    pub ok: bool,
    /// What is wrong and how to fix it, empty when ok.
    pub hint: String,
}

impl Check {
    fn new(name: impl Into<String>, ok: bool, hint: impl Into<String>) -> Self {
        Self { name: name.into(), ok, hint: if ok { String::new() } else { hint.into() } }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Health {
    pub checks: Vec<Check>,
    /// ComfyUI address actually found (useful when auto-detected).
    pub comfy_url: Option<String>,
    pub ollama_models: Vec<String>,
    pub tts_voices: Vec<Voice>,
}

pub async fn check(http: reqwest::Client, s: &Settings) -> Health {
    let mut checks = Vec::new();
    let ollama = Ollama::new(http.clone(), &s.ollama_url);
    let ollama_models = match ollama.models().await {
        Ok(models) => {
            checks.push(Check::new("Ollama", true, ""));
            // `qwen3` and `qwen3:latest` name the same model.
            let has = |name: &str| models.iter().any(|m| m == name || m.strip_suffix(":latest") == Some(name));
            checks.push(Check::new(
                format!("Model {}", s.ollama_model),
                has(&s.ollama_model),
                format!("Chạy: ollama pull {}", s.ollama_model),
            ));
            let vision = s.vision_model();
            let sees = has(vision) && ollama.capabilities(vision).await.is_ok_and(|c| c.iter().any(|c| c == "vision"));
            checks.push(Check::new(
                format!("Soát hình: {vision}"),
                sees,
                format!("'{vision}' không đọc được ảnh hoặc chưa cài. Ví dụ: ollama pull gemma3:12b rồi chọn ở mục Soát hình."),
            ));
            models
        }
        Err(e) => {
            checks.push(Check::new("Ollama", false, e.to_string()));
            Vec::new()
        }
    };

    let comfy_url = match Comfy::connect(http.clone(), &s.comfy_url).await {
        Ok(comfy) => {
            checks.push(Check::new("ComfyUI", true, ""));
            let m = &s.image_model;
            for (node, input, file) in
                [("UNETLoader", "unet_name", &m.unet), ("CLIPLoader", "clip_name", &m.text_encoder), ("VAELoader", "vae_name", &m.vae)]
            {
                let check = match comfy.options(node, input).await {
                    Ok(files) => Check::new(
                        file.clone(),
                        files.contains(file),
                        "Thiếu file trong thư mục ComfyUI/models. Chạy download_models.ps1 rồi khởi động lại ComfyUI.",
                    ),
                    Err(e) => Check::new(file.clone(), false, e.to_string()),
                };
                checks.push(check);
            }
            checks.push(video_models(&comfy).await);
            Some(comfy.base)
        }
        Err(e) => {
            checks.push(Check::new("ComfyUI", false, e.to_string()));
            None
        }
    };

    let tts_voices = match Tts::new(http, &s.tts_url).voices().await {
        Ok(voices) => {
            checks.push(Check::new("VieNeu-TTS", true, ""));
            voices
        }
        Err(e) => {
            checks.push(Check::new("VieNeu-TTS", false, e.to_string()));
            Vec::new()
        }
    };
    let ffmpeg = ffmpeg::version(&s.ffmpeg).await;
    checks.push(Check::new("ffmpeg", ffmpeg.is_ok(), ffmpeg.err().map(|e| e.to_string()).unwrap_or_default()));

    Health { checks, comfy_url, ollama_models, tts_voices }
}

/// One line for all the clip models: they are optional, and eight red lines would bury the checks that matter.
async fn video_models(comfy: &Comfy) -> Check {
    let needed = [
        ("UNETLoader", "unet_name", WAN_HIGH),
        ("UNETLoader", "unet_name", WAN_LOW),
        ("LoraLoaderModelOnly", "lora_name", WAN_LORA_HIGH),
        ("LoraLoaderModelOnly", "lora_name", WAN_LORA_LOW),
        ("CLIPLoader", "clip_name", WAN_TEXT_ENCODER),
        ("VAELoader", "vae_name", WAN_VAE),
        ("UpscaleModelLoader", "model_name", UPSCALER),
        ("FrameInterpolationModelLoader", "model_name", RIFE),
    ];
    let mut missing = Vec::new();
    for (node, input, file) in needed {
        match comfy.options(node, input).await {
            Ok(files) if files.iter().any(|f| f == file) => {}
            Ok(_) => missing.push(file.to_owned()),
            Err(e) => return Check::new("Chuyển động AI (Wan 2.2, tuỳ chọn)", false, e.to_string()),
        }
    }
    Check::new(
        "Chuyển động AI (Wan 2.2, tuỳ chọn)",
        missing.is_empty(),
        format!(
            "Thiếu {}. Chạy scripts/download-models.ps1 -Video (khoảng 38 GB) rồi khởi động lại ComfyUI. Không có vẫn dựng phim zoom/pan bình thường.",
            missing.join(", ")
        ),
    )
}
