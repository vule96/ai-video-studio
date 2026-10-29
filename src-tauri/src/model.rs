//! Project data as stored in `project.json` and shared with the UI (TS types via ts-rs).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum Format {
    Landscape,
    Portrait,
}

impl Format {
    /// Output size for shots: multiples of 16, about one megapixel.
    pub const fn shot_size(self) -> (u32, u32) {
        match self {
            Self::Landscape => (1344, 768),
            Self::Portrait => (768, 1344),
        }
    }

    /// Rendered video size: Full HD in the same orientation.
    pub const fn video_size(self) -> (u32, u32) {
        match self {
            Self::Landscape => (1920, 1080),
            Self::Portrait => (1080, 1920),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum SourceKind {
    Prompt,
    Pdf,
}

/// One generated image. `file` is relative to the project folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Take {
    pub id: String,
    pub file: String,
    pub seed: u32,
    /// Unix time in milliseconds.
    #[ts(type = "number")]
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Character {
    pub id: String,
    pub name: String,
    /// Fixed physical description in English, reused in every prompt.
    pub description: String,
    pub takes: Vec<Take>,
    /// Id of the take used as the reference image in shots.
    pub selected_take: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Shot {
    pub id: String,
    pub title: String,
    /// Ids of visible characters; their order is the order of the reference images.
    pub characters: Vec<String>,
    pub framing: String,
    pub prompt: String,
    pub narration: String,
    pub motion: String,
    pub takes: Vec<Take>,
    pub selected_take: Option<String>,
    /// Latest automatic check of a frame; it applies only while `take_id` is still the selected take.
    #[serde(default)]
    pub review: Option<Review>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Review {
    pub take_id: String,
    pub ok: bool,
    /// Problems the vision model saw, in Vietnamese.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Project {
    pub id: String,
    pub title: String,
    pub source_kind: SourceKind,
    /// File name or the start of the prompt, shown in the UI.
    pub source_label: String,
    pub language: String,
    pub format: Format,
    /// One sentence describing the visual style, shared by every image.
    pub style: String,
    pub characters: Vec<Character>,
    pub shots: Vec<Shot>,
    /// Latest rendered video; `default` keeps project.json files from v0.1 loading.
    #[serde(default)]
    pub video: Option<Video>,
    /// Background music for the render, if the user picked a file.
    #[serde(default)]
    pub music: Option<Music>,
    #[ts(type = "number")]
    pub created_at: u64,
    #[ts(type = "number")]
    pub updated_at: u64,
}

/// A rendered film. Paths are relative to the project folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Video {
    pub file: String,
    /// `.srt` next to the video, one cue per narrated shot.
    pub subtitles: String,
    pub voice: String,
    #[serde(default)]
    pub burned_subtitles: bool,
    #[ts(type = "number")]
    pub duration_ms: u64,
    #[ts(type = "number")]
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Music {
    /// Project-relative copy of the file, e.g. `music/music.mp3`.
    pub file: String,
    /// Original file name, shown in the UI.
    pub name: String,
}

/// More references per image and FLUX starts mixing faces up.
pub const MAX_CHARACTERS_IN_SHOT: usize = 3;

impl Project {
    pub fn character_mut(&mut self, id: &str) -> Option<&mut Character> {
        self.characters.iter_mut().find(|c| c.id == id)
    }

    pub fn shot_mut(&mut self, id: &str) -> Option<&mut Shot> {
        self.shots.iter_mut().find(|s| s.id == id)
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub source_label: String,
    pub shot_count: u32,
    /// Project-relative file of a selected shot image, for the card thumbnail.
    pub cover: Option<String>,
    #[ts(type = "number")]
    pub updated_at: u64,
}

impl From<&Project> for ProjectSummary {
    fn from(p: &Project) -> Self {
        let cover = p.shots.iter().find_map(|s| selected(&s.takes, s.selected_take.as_ref()).map(|t| t.file.clone()));
        Self {
            id: p.id.clone(),
            title: p.title.clone(),
            source_label: p.source_label.clone(),
            shot_count: u32::try_from(p.shots.len()).unwrap_or(u32::MAX),
            cover,
            updated_at: p.updated_at,
        }
    }
}

/// Input for creating a project. `text` is the prompt, or the text extracted from the PDF.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct NewProject {
    pub source_kind: SourceKind,
    pub source_label: String,
    pub text: String,
    pub language: String,
    pub format: Format,
    pub shot_count: u32,
}

/// Edits the UI can make. Each one is applied under the project lock, so it never
/// overwrites takes a running job adds at the same time.
#[derive(Debug, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum ProjectPatch {
    Meta {
        title: String,
        style: String,
        format: Format,
    },
    Character {
        id: String,
        name: String,
        description: String,
    },
    Shot {
        id: String,
        title: String,
        characters: Vec<String>,
        framing: String,
        prompt: String,
        narration: String,
        motion: String,
    },
    SelectTake {
        target: TakeTarget,
        take_id: String,
    },
    /// New empty shot right after `after` (or at the start when `None`).
    AddShot {
        after: Option<String>,
    },
    DeleteShot {
        id: String,
    },
    /// Moves a shot one place up (`-1`) or down (`1`).
    MoveShot {
        id: String,
        by: i32,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum TakeTarget {
    Character(String),
    Shot(String),
}

impl Project {
    /// Applies a UI edit. Input comes from the webview, so ids and references are checked.
    pub fn apply(&mut self, patch: ProjectPatch) -> AppResult<()> {
        match patch {
            ProjectPatch::Meta { title, style, format } => {
                self.title = non_empty(title, "Tên dự án")?;
                self.style = style.trim().to_owned();
                self.format = format;
            }
            ProjectPatch::Character { id, name, description } => {
                let c = self.character_mut(&id).ok_or_else(|| AppError::not_found(format!("Không có nhân vật '{id}'")))?;
                c.name = non_empty(name, "Tên nhân vật")?;
                c.description = description.trim().to_owned();
            }
            ProjectPatch::Shot { id, title, characters, framing, prompt, narration, motion } => {
                if characters.len() > MAX_CHARACTERS_IN_SHOT {
                    return Err(AppError::invalid(format!("Mỗi shot tối đa {MAX_CHARACTERS_IN_SHOT} nhân vật")));
                }
                if let Some(unknown) = characters.iter().find(|c| !self.characters.iter().any(|k| &k.id == *c)) {
                    return Err(AppError::invalid(format!("Shot '{id}' tham chiếu nhân vật không tồn tại: '{unknown}'")));
                }
                let s = self.shot_mut(&id).ok_or_else(|| AppError::not_found(format!("Không có shot '{id}'")))?;
                s.title = title.trim().to_owned();
                s.characters = characters;
                s.framing = framing.trim().to_owned();
                s.prompt = prompt.trim().to_owned();
                s.narration = narration.trim().to_owned();
                s.motion = motion.trim().to_owned();
            }
            ProjectPatch::SelectTake { target, take_id } => {
                let (takes, selected) = self.takes_mut(&target)?;
                if !takes.iter().any(|t| t.id == take_id) {
                    return Err(AppError::not_found(format!("Không có ảnh '{take_id}'")));
                }
                *selected = Some(take_id);
            }
            ProjectPatch::AddShot { after } => {
                let at = match after {
                    Some(id) => self.shot_index(&id)? + 1,
                    None => 0,
                };
                let shot = Shot {
                    id: self.next_shot_id(),
                    title: "Shot mới".into(),
                    characters: vec![],
                    framing: "medium shot".into(),
                    prompt: String::new(),
                    narration: String::new(),
                    motion: "slow push-in".into(),
                    takes: vec![],
                    selected_take: None,
                    review: None,
                };
                self.shots.insert(at, shot);
            }
            ProjectPatch::DeleteShot { id } => {
                // shortcut: the shot's images stay on disk; a cleanup pass can remove unreferenced files later.
                let i = self.shot_index(&id)?;
                self.shots.remove(i);
            }
            ProjectPatch::MoveShot { id, by } => {
                let i = self.shot_index(&id)?;
                let j = i.checked_add_signed(isize::try_from(by).unwrap_or(0)).filter(|j| *j < self.shots.len());
                let Some(j) = j else { return Err(AppError::invalid("Shot đã ở đầu hoặc cuối danh sách")) };
                self.shots.swap(i, j);
            }
        }
        Ok(())
    }

    fn shot_index(&self, id: &str) -> AppResult<usize> {
        self.shots.iter().position(|s| s.id == id).ok_or_else(|| AppError::not_found(format!("Không có shot '{id}'")))
    }

    /// Ids are identity, not position: after moves they no longer follow the order, so take one past the highest.
    fn next_shot_id(&self) -> String {
        let max = self.shots.iter().filter_map(|s| s.id.strip_prefix('S')?.parse::<u32>().ok()).max().unwrap_or(0);
        format!("S{:02}", max + 1)
    }

    pub fn takes_mut(&mut self, target: &TakeTarget) -> AppResult<(&mut Vec<Take>, &mut Option<String>)> {
        match target {
            TakeTarget::Character(id) => self
                .character_mut(id)
                .map(|c| (&mut c.takes, &mut c.selected_take))
                .ok_or_else(|| AppError::not_found(format!("Không có nhân vật '{id}'"))),
            TakeTarget::Shot(id) => self
                .shot_mut(id)
                .map(|s| (&mut s.takes, &mut s.selected_take))
                .ok_or_else(|| AppError::not_found(format!("Không có shot '{id}'"))),
        }
    }

    /// Adds a take. Shots switch to the new take (a regeneration should show up right away);
    /// characters keep the reference the user picked and only auto-select their first take.
    pub fn push_take(&mut self, target: &TakeTarget, take: Take) -> AppResult<()> {
        let select = matches!(target, TakeTarget::Shot(_));
        let (takes, selected) = self.takes_mut(target)?;
        if select || selected.is_none() {
            *selected = Some(take.id.clone());
        }
        takes.push(take);
        Ok(())
    }
}

/// The take a character or shot currently uses.
pub fn selected<'a>(takes: &'a [Take], selected: Option<&String>) -> Option<&'a Take> {
    let id = selected?;
    takes.iter().find(|t| &t.id == id)
}

fn non_empty(s: String, field: &str) -> AppResult<String> {
    let s = s.trim().to_owned();
    if s.is_empty() {
        return Err(AppError::invalid(format!("{field} không được để trống")));
    }
    Ok(s)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(ids: &[&str]) -> Project {
        let shot = |id: &str| Shot {
            id: id.into(),
            title: String::new(),
            characters: vec![],
            framing: String::new(),
            prompt: String::new(),
            narration: String::new(),
            motion: String::new(),
            takes: vec![],
            selected_take: None,
            review: None,
        };
        Project {
            id: "p".into(),
            title: "t".into(),
            source_kind: SourceKind::Prompt,
            source_label: String::new(),
            language: "vi".into(),
            format: Format::Landscape,
            style: String::new(),
            characters: vec![],
            shots: ids.iter().map(|id| shot(id)).collect(),
            video: None,
            music: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn order(p: &Project) -> Vec<&str> {
        p.shots.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn add_move_delete_shots() {
        let mut p = project(&["S01", "S02", "S03"]);
        p.apply(ProjectPatch::MoveShot { id: "S03".into(), by: -1 }).unwrap();
        assert_eq!(order(&p), ["S01", "S03", "S02"]);
        p.apply(ProjectPatch::AddShot { after: Some("S01".into()) }).unwrap();
        assert_eq!(order(&p), ["S01", "S04", "S03", "S02"]);
        p.apply(ProjectPatch::AddShot { after: None }).unwrap();
        assert_eq!(order(&p)[0], "S05");
        p.apply(ProjectPatch::DeleteShot { id: "S03".into() }).unwrap();
        assert_eq!(order(&p), ["S05", "S01", "S04", "S02"]);
        assert!(p.apply(ProjectPatch::MoveShot { id: "S02".into(), by: 1 }).is_err());
        assert!(p.apply(ProjectPatch::MoveShot { id: "S05".into(), by: -1 }).is_err());
        assert!(p.apply(ProjectPatch::DeleteShot { id: "S99".into() }).is_err());
    }
}
