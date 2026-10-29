//! Tauri commands: thin glue between the webview and the store / job queue.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::health::{self, Health};
use crate::jobs::{JobInfo, JobSpec, Jobs};
use crate::model::{Music, NewProject, Project, ProjectPatch, ProjectSummary};
use crate::pipeline::text::clean_text;
use crate::settings::Settings;
use crate::store::ProjectStore;

/// shortcut: caps source text at ~2 MB (a long novel); raise it if whole series need to fit.
const MAX_SOURCE_BYTES: usize = 2_000_000;
/// A long track at high bitrate; anything bigger is probably a video file picked by mistake.
const MAX_MUSIC_BYTES: usize = 60_000_000;
const MUSIC_EXTENSIONS: [&str; 6] = ["mp3", "m4a", "aac", "wav", "ogg", "flac"];

pub struct AppState {
    pub store: Arc<ProjectStore>,
    pub settings: Arc<RwLock<Settings>>,
    pub settings_path: PathBuf,
    pub http: reqwest::Client,
    pub jobs: Jobs,
}

impl AppState {
    fn settings(&self) -> Settings {
        self.settings.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

#[tauri::command]
pub fn projects_root(state: State<'_, AppState>) -> String {
    state.store.root().to_string_lossy().into_owned()
}

#[tauri::command]
pub async fn projects_list(state: State<'_, AppState>) -> AppResult<Vec<ProjectSummary>> {
    state.store.list().await
}

#[tauri::command]
pub async fn project_get(state: State<'_, AppState>, id: String) -> AppResult<Project> {
    state.store.get(&id).await
}

/// Creates the project and queues the storyboard job right away.
#[tauri::command]
pub async fn project_create(state: State<'_, AppState>, mut input: NewProject) -> AppResult<Project> {
    input.text = clean_text(&input.text);
    if input.text.is_empty() {
        return Err(AppError::invalid("Nội dung trống. Nhập prompt hoặc chọn file PDF có chữ (PDF scan chưa được hỗ trợ)."));
    }
    if input.text.len() > MAX_SOURCE_BYTES {
        return Err(AppError::invalid("Nội dung quá dài. Hãy chọn một khoảng trang nhỏ hơn."));
    }
    input.source_label = input.source_label.trim().chars().take(120).collect();
    if input.source_label.is_empty() {
        input.source_label = "Dự án mới".into();
    }
    let project = state.store.create(&input).await?;
    state.jobs.enqueue(JobSpec::Storyboard { project_id: project.id.clone(), shot_count: input.shot_count })?;
    Ok(project)
}

#[tauri::command]
pub async fn project_patch(state: State<'_, AppState>, id: String, patch: ProjectPatch) -> AppResult<Project> {
    Ok(state.store.update(&id, |p| p.apply(patch)).await?.0)
}

/// Background music upload. The file arrives as the raw request body (no JSON number arrays for megabytes of audio);
/// project id and the percent-encoded file name come as headers.
#[tauri::command]
pub async fn project_music_set(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> AppResult<Project> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::invalid("Thiếu dữ liệu file nhạc"));
    };
    let header = |name: &str| request.headers().get(name).and_then(|v| v.to_str().ok()).map(percent_decode);
    let id = header("project-id").ok_or_else(|| AppError::invalid("Thiếu mã dự án"))?;
    let name = header("file-name").unwrap_or_default();
    if bytes.len() > MAX_MUSIC_BYTES {
        return Err(AppError::invalid("File nhạc quá lớn (tối đa 60 MB)"));
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if !MUSIC_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::invalid(format!("Chỉ nhận file nhạc {}", MUSIC_EXTENSIONS.join(", "))));
    }
    let file = format!("music/music.{ext}");
    state.store.write_file(&id, &file, bytes).await?;
    let music = Music { file: file.clone(), name: name.chars().take(120).collect() };
    let (project, old) = state.store.update(&id, |p| Ok(p.music.replace(music))).await?;
    remove_replaced(&state, &id, old, &file).await;
    Ok(project)
}

#[tauri::command]
pub async fn project_music_remove(state: State<'_, AppState>, id: String) -> AppResult<Project> {
    let (project, old) = state.store.update(&id, |p| Ok(p.music.take())).await?;
    remove_replaced(&state, &id, old, "").await;
    Ok(project)
}

/// Best effort: a leftover music file only wastes disk.
async fn remove_replaced(state: &AppState, id: &str, old: Option<Music>, keep: &str) {
    if let (Some(old), Ok(dir)) = (old, state.store.dir(id))
        && old.file != keep
    {
        let _ = tokio::fs::remove_file(dir.join(old.file)).await;
    }
}

/// Decodes `%XX` escapes from `encodeURIComponent`; invalid escapes are kept as typed.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tauri::command]
pub async fn project_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if state.jobs.is_busy(&id) {
        return Err(AppError::invalid("Dự án đang có job chạy. Huỷ job trước khi xoá."));
    }
    state.store.delete(&id).await
}

#[tauri::command]
pub async fn job_enqueue(state: State<'_, AppState>, spec: JobSpec) -> AppResult<JobInfo> {
    // Check the project exists now, so a typo fails here instead of later in the queue.
    state.store.get(spec.project_id()).await?;
    state.jobs.enqueue(spec)
}

#[tauri::command]
pub fn jobs_list(state: State<'_, AppState>) -> Vec<JobInfo> {
    state.jobs.list()
}

#[tauri::command]
pub fn job_cancel(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.jobs.cancel(&id)
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn settings_save(state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    settings.save(&state.settings_path)?;
    *state.settings.write().unwrap_or_else(std::sync::PoisonError::into_inner) = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub async fn health_check(state: State<'_, AppState>) -> AppResult<Health> {
    Ok(health::check(state.http.clone(), &state.settings()).await)
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn decodes_encode_uri_component() {
        assert_eq!(percent_decode("Nh%E1%BA%A1c%20n%E1%BB%81n.mp3"), "Nhạc nền.mp3");
        assert_eq!(percent_decode("100%.mp3"), "100%.mp3");
        assert_eq!(percent_decode("a%zz"), "a%zz");
    }
}
