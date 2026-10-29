use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Image model files, as named in ComfyUI's `models/` folders.
/// Defaults: FLUX.2 [klein] 4B distilled FP8 (Apache 2.0), 4 steps, no CFG; fits a 16 GB card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ImageModel {
    pub unet: String,
    pub text_encoder: String,
    pub vae: String,
    pub steps: u32,
    pub cfg: f32,
}

impl Default for ImageModel {
    fn default() -> Self {
        Self {
            unet: "flux-2-klein-4b-fp8.safetensors".into(),
            text_encoder: "qwen_3_4b.safetensors".into(),
            vae: "flux2-vae.safetensors".into(),
            steps: 4,
            cfg: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Settings {
    pub ollama_url: String,
    pub ollama_model: String,
    /// Model that can look at images, for frame review. Empty: use `ollama_model`.
    pub vision_model: String,
    /// Empty: try ComfyUI portable (8188), then ComfyUI Desktop (8000).
    pub comfy_url: String,
    pub image_model: ImageModel,
    /// VieNeu-TTS server. 8001 rather than its default 8000, which ComfyUI Desktop uses.
    pub tts_url: String,
    /// `ffmpeg` on PATH, or a full path to the executable.
    pub ffmpeg: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".into(),
            ollama_model: "qwen3:8b".into(),
            vision_model: String::new(),
            comfy_url: String::new(),
            image_model: ImageModel::default(),
            tts_url: "http://127.0.0.1:8001".into(),
            ffmpeg: "ffmpeg".into(),
        }
    }
}

impl Settings {
    /// Missing or unreadable file falls back to defaults; the app must still start.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> AppResult<()> {
        self.validate()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    fn validate(&self) -> AppResult<()> {
        if self.tts_url.trim().is_empty() || self.ffmpeg.trim().is_empty() {
            return Err(AppError::invalid("Địa chỉ VieNeu-TTS và đường dẫn ffmpeg không được để trống"));
        }
        for (name, url) in [("Ollama", &self.ollama_url), ("ComfyUI", &self.comfy_url), ("VieNeu-TTS", &self.tts_url)] {
            if !url.is_empty() && !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(AppError::invalid(format!("Địa chỉ {name} phải bắt đầu bằng http:// hoặc https://")));
            }
        }
        if self.ollama_model.trim().is_empty() {
            return Err(AppError::invalid("Chưa chọn model Ollama"));
        }
        if !(1..=100).contains(&self.image_model.steps) {
            return Err(AppError::invalid("Số bước lấy mẫu phải từ 1 đến 100"));
        }
        Ok(())
    }
}

impl Settings {
    pub fn vision_model(&self) -> &str {
        if self.vision_model.trim().is_empty() { &self.ollama_model } else { self.vision_model.trim() }
    }
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_file_keeps_defaults_for_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(dir.path());
        std::fs::write(&path, r#"{"ollamaModel":"qwen3:14b"}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.ollama_model, "qwen3:14b");
        assert_eq!(s.image_model, ImageModel::default());
    }

    #[test]
    fn rejects_bad_url() {
        let s = Settings { comfy_url: "127.0.0.1:8188".into(), ..Settings::default() };
        assert!(s.save(&tempfile::tempdir().unwrap().path().join("s.json")).is_err());
    }
}
