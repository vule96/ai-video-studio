//! Background jobs. One worker runs them strictly one at a time: most jobs use the GPU
//! (Ollama or ComfyUI), and two models at once would not fit in 16 GB of VRAM.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::model::{Character, Format, Take, TakeTarget, Video, now_ms, selected};
use crate::pipeline::prompts::{CHARACTER_SIZE, character_prompt, shot_prompt};
use crate::pipeline::render::{self, Clip, Finish};
use crate::pipeline::workflow::{ImageRequest, image_graph};
use crate::pipeline::{review, storyboard};
use crate::services::comfy::Comfy;
use crate::services::ollama::{Ollama, Prompt};
use crate::services::tts::Tts;
use crate::services::{cancellable, ffmpeg};
use crate::settings::Settings;
use crate::store::ProjectStore;

/// `(done, total, message)`, reported between steps of a job.
type Progress<'a> = &'a (dyn Fn(u32, u32, &str) + Sync);

/// Finished jobs kept for the UI; older ones are dropped so the list does not grow forever.
const KEEP_FINISHED: usize = 30;

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum JobSpec {
    Storyboard { project_id: String, shot_count: u32 },
    CharacterTakes { project_id: String, character_id: String, count: u32 },
    ShotTakes { project_id: String, shot_ids: Vec<String> },
    Render { project_id: String, voice: String, burn_subtitles: bool },
    Review { project_id: String, shot_ids: Vec<String> },
}

impl JobSpec {
    pub fn project_id(&self) -> &str {
        match self {
            Self::Storyboard { project_id, .. }
            | Self::CharacterTakes { project_id, .. }
            | Self::ShotTakes { project_id, .. }
            | Self::Render { project_id, .. }
            | Self::Review { project_id, .. } => project_id,
        }
    }

    fn label(&self) -> String {
        match self {
            Self::Storyboard { shot_count, .. } => format!("Viết storyboard ({shot_count} shot)"),
            Self::CharacterTakes { character_id, count, .. } => format!("Sinh {count} ảnh nhân vật '{character_id}'"),
            Self::ShotTakes { shot_ids, .. } => format!("Sinh khung hình {}", shot_ids.join(", ")),
            Self::Render { voice, .. } => format!("Dựng phim, giọng {voice}"),
            Self::Review { shot_ids, .. } => format!("Soát {} khung hình", shot_ids.len()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct JobInfo {
    pub id: String,
    pub project_id: String,
    pub spec: JobSpec,
    pub label: String,
    pub status: JobStatus,
    pub done: u32,
    pub total: u32,
    pub message: String,
    pub error: Option<String>,
}

/// Where job updates go: the Tauri window in the app, a recorder in tests.
pub trait EventSink: Send + Sync + 'static {
    fn job(&self, info: &JobInfo);
    fn project_changed(&self, project_id: &str);
}

/// Shared services every job needs.
pub struct Context {
    pub store: Arc<ProjectStore>,
    pub settings: Arc<RwLock<Settings>>,
    pub http: reqwest::Client,
    pub sink: Box<dyn EventSink>,
}

impl Context {
    fn settings(&self) -> Settings {
        self.settings.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

struct Entry {
    info: JobInfo,
    cancel: CancellationToken,
}

#[derive(Clone)]
pub struct Jobs {
    entries: Arc<Mutex<Vec<Entry>>>,
    tx: mpsc::UnboundedSender<String>,
    ctx: Arc<Context>,
}

impl Jobs {
    /// Starts the worker on the current Tokio runtime.
    pub fn start(ctx: Context) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let jobs = Self { entries: Arc::default(), tx, ctx: Arc::new(ctx) };
        tauri::async_runtime::spawn(jobs.clone().worker(rx));
        jobs
    }

    pub fn enqueue(&self, spec: JobSpec) -> AppResult<JobInfo> {
        let info = JobInfo {
            id: uuid::Uuid::new_v4().simple().to_string()[..10].to_owned(),
            project_id: spec.project_id().to_owned(),
            label: spec.label(),
            spec,
            status: JobStatus::Queued,
            done: 0,
            total: 0,
            message: "Đang chờ".into(),
            error: None,
        };
        self.with_entries(|e| e.push(Entry { info: info.clone(), cancel: CancellationToken::new() }));
        self.tx.send(info.id.clone()).map_err(|_| AppError::invalid("Bộ xử lý job đã dừng"))?;
        self.ctx.sink.job(&info);
        Ok(info)
    }

    /// True while the project has a queued or running job.
    pub fn is_busy(&self, project_id: &str) -> bool {
        self.with_entries(|e| {
            e.iter().any(|x| x.info.project_id == project_id && matches!(x.info.status, JobStatus::Queued | JobStatus::Running))
        })
    }

    pub fn list(&self) -> Vec<JobInfo> {
        self.with_entries(|e| e.iter().map(|x| x.info.clone()).collect())
    }

    /// Queued jobs are cancelled immediately; a running job stops at its next check.
    pub fn cancel(&self, id: &str) -> AppResult<()> {
        let info = self.with_entries(|entries| {
            let entry = entries.iter_mut().find(|x| x.info.id == id)?;
            entry.cancel.cancel();
            (entry.info.status == JobStatus::Queued).then(|| {
                entry.info.status = JobStatus::Cancelled;
                entry.info.message = "Đã huỷ".into();
                entry.info.clone()
            })
        });
        if let Some(info) = info {
            self.ctx.sink.job(&info);
        }
        Ok(())
    }

    async fn worker(self, mut rx: mpsc::UnboundedReceiver<String>) {
        while let Some(id) = rx.recv().await {
            let started = self.update(&id, |i| {
                if i.status != JobStatus::Queued {
                    return false;
                }
                i.status = JobStatus::Running;
                i.message = "Đang chạy".into();
                true
            });
            let Some((spec, cancel)) = started.filter(|s| *s).and_then(|_| self.spec_and_token(&id)) else { continue };

            let progress = |done: u32, total: u32, message: &str| {
                self.update(&id, |i| {
                    (i.done, i.total, i.message) = (done, total, message.to_owned());
                });
            };
            let result = self.run(&spec, &cancel, &progress).await;
            self.update(&id, |i| match result {
                Ok(()) => (i.status, i.message) = (JobStatus::Done, "Xong".into()),
                Err(AppError::Cancelled) => (i.status, i.message) = (JobStatus::Cancelled, "Đã huỷ".into()),
                Err(e) => {
                    (i.status, i.message) = (JobStatus::Failed, "Lỗi".into());
                    i.error = Some(e.to_string());
                }
            });
            self.prune();
        }
    }

    async fn run(&self, spec: &JobSpec, cancel: &CancellationToken, progress: Progress<'_>) -> AppResult<()> {
        let ctx = &*self.ctx;
        match spec {
            JobSpec::Storyboard { project_id, shot_count } => run_storyboard(ctx, project_id, *shot_count, cancel, progress).await,
            JobSpec::CharacterTakes { project_id, character_id, count } => {
                run_character_takes(ctx, project_id, character_id, *count, cancel, progress).await
            }
            JobSpec::ShotTakes { project_id, shot_ids } => run_shot_takes(ctx, project_id, shot_ids, cancel, progress).await,
            JobSpec::Render { project_id, voice, burn_subtitles } => {
                run_render(ctx, project_id, voice, *burn_subtitles, cancel, progress).await
            }
            JobSpec::Review { project_id, shot_ids } => run_review(ctx, project_id, shot_ids, cancel, progress).await,
        }
    }

    /// Mutates a job, emits the new state, returns what `f` returned.
    fn update<R>(&self, id: &str, f: impl FnOnce(&mut JobInfo) -> R) -> Option<R> {
        let (out, info) = self.with_entries(|entries| {
            let entry = entries.iter_mut().find(|x| x.info.id == id)?;
            Some((f(&mut entry.info), entry.info.clone()))
        })?;
        self.ctx.sink.job(&info);
        Some(out)
    }

    fn spec_and_token(&self, id: &str) -> Option<(JobSpec, CancellationToken)> {
        self.with_entries(|e| e.iter().find(|x| x.info.id == id).map(|x| (x.info.spec.clone(), x.cancel.clone())))
    }

    fn prune(&self) {
        self.with_entries(|entries| {
            let finished = entries.iter().filter(|x| !matches!(x.info.status, JobStatus::Queued | JobStatus::Running)).count();
            let mut drop = finished.saturating_sub(KEEP_FINISHED);
            entries.retain(|x| {
                let old = drop > 0 && !matches!(x.info.status, JobStatus::Queued | JobStatus::Running);
                drop -= usize::from(old);
                !old
            });
        });
    }

    fn with_entries<R>(&self, f: impl FnOnce(&mut Vec<Entry>) -> R) -> R {
        f(&mut self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

fn random_seed() -> u32 {
    // Truncation is the point: any 32 random bits make a seed.
    #[allow(clippy::cast_possible_truncation)]
    let seed = uuid::Uuid::new_v4().as_u128() as u32;
    seed
}

async fn run_storyboard(
    ctx: &Context,
    project_id: &str,
    shot_count: u32,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let settings = ctx.settings();
    // Unload image models first so the LLM gets the GPU. ComfyUI not running is fine.
    if let Ok(comfy) = Comfy::connect(ctx.http.clone(), &settings.comfy_url).await {
        comfy.free().await;
    }
    let project = ctx.store.get(project_id).await?;
    let source = ctx.store.source(project_id).await?;
    let llm = Ollama::new(ctx.http.clone(), &settings.ollama_url);
    let req = storyboard::Request {
        source: &source,
        kind: project.source_kind,
        language: &project.language,
        shot_count: shot_count.clamp(1, 40),
        model: &settings.ollama_model,
    };
    let draft = storyboard::generate(&llm, &req, cancel, progress).await?;
    ctx.store
        .update(project_id, |p| {
            // Keep a title the user typed; replace the placeholder (file name / prompt) with the LLM's.
            if !draft.title.is_empty() && p.title == p.source_label.chars().take(80).collect::<String>() {
                p.title = draft.title;
            }
            p.style = draft.style;
            p.characters = draft.characters;
            p.shots = draft.shots;
            Ok(())
        })
        .await?;
    ctx.sink.project_changed(project_id);
    Ok(())
}

async fn run_character_takes(
    ctx: &Context,
    project_id: &str,
    character_id: &str,
    count: u32,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let settings = ctx.settings();
    let comfy = Comfy::connect(ctx.http.clone(), &settings.comfy_url).await?;
    let project = ctx.store.get(project_id).await?;
    let character = project
        .characters
        .iter()
        .find(|c| c.id == character_id)
        .ok_or_else(|| AppError::not_found(format!("Không có nhân vật '{character_id}'")))?;
    let (width, height) = CHARACTER_SIZE;
    let count = count.clamp(1, 8);
    // Continue the casting rotation after existing takes, so "generate more" shows new faces.
    let first_variant = character.takes.len();
    for i in 0..count {
        progress(i, count, &format!("Ảnh {}/{count}", i + 1));
        let prompt = character_prompt(&project.style, character, first_variant + i as usize);
        let seed = random_seed();
        let req = ImageRequest { prompt: &prompt, width, height, seed, references: &[], filename_prefix: "aivs/character" };
        let bytes = comfy.run(&image_graph(&settings.image_model, &req), cancel).await?;
        save_take(ctx, project_id, TakeTarget::Character(character_id.to_owned()), "characters", seed, &bytes).await?;
    }
    progress(count, count, "Xong");
    Ok(())
}

async fn run_shot_takes(
    ctx: &Context,
    project_id: &str,
    shot_ids: &[String],
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let settings = ctx.settings();
    let project = ctx.store.get(project_id).await?;
    let project_dir = ctx.store.dir(project_id)?;
    let shots = shot_ids
        .iter()
        .map(|id| project.shots.iter().find(|s| &s.id == id).ok_or_else(|| AppError::not_found(format!("Không có shot '{id}'"))))
        .collect::<AppResult<Vec<_>>>()?;

    // Fail before any GPU work if a shot needs a character that has no reference image yet.
    let reference = |id: &str| -> AppResult<(&Character, &Take)> {
        let c = project.characters.iter().find(|c| c.id == id).ok_or_else(|| AppError::not_found(format!("Không có nhân vật '{id}'")))?;
        let take = selected(&c.takes, c.selected_take.as_ref()).ok_or_else(|| {
            AppError::invalid(format!("Nhân vật '{}' chưa có ảnh tham chiếu. Hãy sinh và chọn ảnh ở tab Nhân vật trước.", c.name))
        })?;
        Ok((c, take))
    };
    for shot in &shots {
        for id in &shot.characters {
            reference(id)?;
        }
    }

    let comfy = Comfy::connect(ctx.http.clone(), &settings.comfy_url).await?;
    let mut uploaded: HashMap<String, String> = HashMap::new();
    let (width, height) = project.format.shot_size();
    let total = u32::try_from(shots.len()).unwrap_or(u32::MAX);
    for (i, shot) in (0u32..).zip(&shots) {
        progress(i, total, &format!("Shot {} ({}/{total})", shot.id, i + 1));
        let mut present = Vec::new();
        let mut references = Vec::new();
        for id in &shot.characters {
            let (character, take) = reference(id)?;
            if !uploaded.contains_key(&take.id) {
                let name = comfy.upload_image(&project_dir.join(&take.file), &format!("aivs_{project_id}_{}.png", take.id)).await?;
                uploaded.insert(take.id.clone(), name);
            }
            present.push(character);
            references.push(uploaded[&take.id].clone());
        }
        let prompt = shot_prompt(&project.style, shot, &present);
        let seed = random_seed();
        let req = ImageRequest { prompt: &prompt, width, height, seed, references: &references, filename_prefix: "aivs/shot" };
        let bytes = comfy.run(&image_graph(&settings.image_model, &req), cancel).await?;
        save_take(ctx, project_id, TakeTarget::Shot(shot.id.clone()), "shots", seed, &bytes).await?;
    }
    progress(total, total, "Xong");
    Ok(())
}

async fn save_take(ctx: &Context, project_id: &str, target: TakeTarget, folder: &str, seed: u32, bytes: &[u8]) -> AppResult<()> {
    let owner = match &target {
        TakeTarget::Character(id) | TakeTarget::Shot(id) => id.clone(),
    };
    let take_id = format!("{owner}-{seed}");
    let file = format!("{folder}/{take_id}.png");
    ctx.store.write_file(project_id, &file, bytes).await?;
    let take = Take { id: take_id, file, seed, created_at: now_ms() };
    ctx.store.update(project_id, |p| p.push_take(&target, take)).await?;
    ctx.sink.project_changed(project_id);
    Ok(())
}

/// Narrates every shot with VieNeu-TTS, turns each frame into a clip with a camera move, joins them, then mixes
/// music, normalises loudness and optionally burns in subtitles.
async fn run_render(
    ctx: &Context,
    project_id: &str,
    voice: &str,
    burn_subtitles: bool,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    const TMP: &str = "render/tmp";
    let settings = ctx.settings();
    let project = ctx.store.get(project_id).await?;
    let dir = ctx.store.dir(project_id)?;
    if project.shots.is_empty() {
        return Err(AppError::invalid("Dự án chưa có shot nào"));
    }
    let missing: Vec<&str> =
        project.shots.iter().filter(|s| selected(&s.takes, s.selected_take.as_ref()).is_none()).map(|s| s.id.as_str()).collect();
    if !missing.is_empty() {
        return Err(AppError::invalid(format!("Shot {} chưa có khung hình. Vẽ ở bước Khung hình trước.", missing.join(", "))));
    }
    // Fail before spending minutes on narration if ffmpeg is not installed.
    ffmpeg::version(&settings.ffmpeg).await?;
    if let Some(music) = &project.music
        && tokio::fs::metadata(dir.join(&music.file)).await.is_err()
    {
        return Err(AppError::not_found(format!("Không thấy file nhạc nền '{}'. Chọn lại nhạc hoặc bỏ nhạc nền.", music.name)));
    }

    let n = u32::try_from(project.shots.len()).unwrap_or(u32::MAX);
    let total = 2 * n + 2;
    let tts = Tts::new(ctx.http.clone(), &settings.tts_url);
    // (project-relative WAV, seconds) per shot; no file for a shot without narration.
    let mut narration: Vec<(Option<String>, f64)> = Vec::new();
    for (i, shot) in (0u32..).zip(&project.shots) {
        progress(i, total, &format!("Lồng tiếng {}", shot.id));
        let text = shot.narration.trim();
        if text.is_empty() {
            narration.push((None, 0.0));
            continue;
        }
        let rel = format!("render/audio/{}.wav", render::audio_key(voice, text));
        let len = match tokio::fs::metadata(dir.join(&rel)).await {
            Ok(meta) => meta.len(),
            Err(_) => {
                let pcm = cancellable(cancel, tts.speak(text, voice)).await?;
                let file = render::wav(&pcm);
                ctx.store.write_file(project_id, &rel, &file).await?;
                file.len() as u64
            }
        };
        narration.push((Some(rel), render::wav_secs(len)));
    }

    tokio::fs::create_dir_all(dir.join(TMP)).await?;
    let mut clips = Vec::new();
    let mut cues = Vec::new();
    let mut start = 0.0;
    for ((i, shot), (audio, secs)) in (0u32..).zip(&project.shots).zip(&narration) {
        progress(n + i, total, &format!("Dựng {}", shot.id));
        let image = selected(&shot.takes, shot.selected_take.as_ref()).map(|t| t.file.as_str()).unwrap_or_default();
        let name = format!("{i:03}.mp4");
        let out = format!("{TMP}/{name}");
        let clip = Clip {
            image,
            audio: audio.as_deref(),
            audio_secs: *secs,
            motion: render::motion(&shot.motion),
            size: project.format.video_size(),
            out: &out,
        };
        ffmpeg::run(&settings.ffmpeg, &dir, &render::clip_args(&clip), cancel).await?;
        cues.extend(render::cues(start + render::LEAD, *secs, &shot.narration));
        start += render::frames_secs(render::clip_frames(*secs));
        clips.push(name);
    }

    progress(2 * n, total, "Ghép phim");
    ctx.store.write_file(project_id, &format!("{TMP}/list.txt"), render::concat_list(&clips).as_bytes()).await?;
    let joined = format!("{TMP}/joined.mp4");
    ffmpeg::run(&settings.ffmpeg, &dir, &render::concat_args(&format!("{TMP}/list.txt"), &joined), cancel).await?;

    progress(2 * n + 1, total, if project.music.is_some() { "Trộn nhạc, chuẩn hoá âm lượng" } else { "Chuẩn hoá âm lượng" });
    let stem = format!("render/video-{}", now_ms());
    let video = Video {
        file: format!("{stem}.mp4"),
        subtitles: format!("{stem}.srt"),
        voice: voice.to_owned(),
        burned_subtitles: burn_subtitles,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        duration_ms: (start * 1000.0).round() as u64,
        created_at: now_ms(),
    };
    // The .srt is written first: the burn-in step reads it.
    ctx.store.write_file(project_id, &video.subtitles, render::srt(&cues).as_bytes()).await?;
    let finish = Finish {
        joined: &joined,
        music: project.music.as_ref().map(|m| m.file.as_str()),
        subtitles: burn_subtitles.then_some(video.subtitles.as_str()),
        portrait: project.format == Format::Portrait,
        secs: start,
        out: &video.file,
    };
    ffmpeg::run(&settings.ffmpeg, &dir, &render::finish_args(&finish), cancel).await?;

    let (_, old) = ctx.store.update(project_id, |p| Ok(p.video.replace(video))).await?;
    // Cleanup is best effort: a leftover file wastes disk, it does not break anything.
    if let Some(old) = old {
        let _ = tokio::fs::remove_file(dir.join(old.file)).await;
        let _ = tokio::fs::remove_file(dir.join(old.subtitles)).await;
    }
    let _ = tokio::fs::remove_dir_all(dir.join(TMP)).await;
    ctx.sink.project_changed(project_id);
    progress(total, total, "Xong");
    Ok(())
}

/// Shows each selected frame to a vision model with the cast and description it was drawn from.
async fn run_review(
    ctx: &Context,
    project_id: &str,
    shot_ids: &[String],
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let settings = ctx.settings();
    let model = settings.vision_model().to_owned();
    // The vision model needs the GPU that ComfyUI may still be holding.
    if let Ok(comfy) = Comfy::connect(ctx.http.clone(), &settings.comfy_url).await {
        comfy.free().await;
    }
    let project = ctx.store.get(project_id).await?;
    let dir = ctx.store.dir(project_id)?;
    let llm = Ollama::new(ctx.http.clone(), &settings.ollama_url);
    let schema = review::schema();
    let shots: Vec<_> = project.shots.iter().filter(|s| shot_ids.contains(&s.id)).collect();
    let total = u32::try_from(shots.len()).unwrap_or(u32::MAX);
    let result = async {
        for (i, shot) in (0u32..).zip(&shots) {
            let Some(take) = selected(&shot.takes, shot.selected_take.as_ref()) else { continue };
            progress(i, total, &format!("Soát {} ({}/{total})", shot.id, i + 1));
            let cast: Vec<&Character> = shot.characters.iter().filter_map(|id| project.characters.iter().find(|c| &c.id == id)).collect();
            let image = review::base64(&tokio::fs::read(dir.join(&take.file)).await?);
            let user = review::user_prompt(shot, &cast);
            let images = [image];
            let prompt = Prompt { images: &images, schema: Some(&schema), keep_loaded: true, ..Prompt::new(review::SYSTEM_PROMPT, &user) };
            let raw = llm.chat_json(&model, &prompt, cancel).await?;
            let result = review::parse(&raw, &take.id);
            let shot_id = shot.id.clone();
            ctx.store
                .update(project_id, |p| {
                    // The shot may have been deleted meanwhile; then there is nothing to annotate.
                    if let Some(s) = p.shot_mut(&shot_id) {
                        s.review = Some(result);
                    }
                    Ok(())
                })
                .await?;
            ctx.sink.project_changed(project_id);
        }
        Ok(())
    }
    .await;
    llm.unload(&model).await;
    result.map_err(|e: AppError| match e {
        AppError::Service { message, .. } if message.contains("image") || message.contains("vision") => AppError::service(
            "Ollama",
            format!(
                "{message}\n→ Model '{model}' không đọc được ảnh. Chọn model có thị giác (ví dụ gemma3:12b) ở mục Soát hình trong Cài đặt."
            ),
        ),
        other => other,
    })?;
    progress(total, total, "Xong");
    Ok(())
}
