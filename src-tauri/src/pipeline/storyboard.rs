//! Source text → storyboard (style, characters with fixed looks, shots) via a local LLM.

use std::collections::HashSet;

use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::text::{chunk_text, slug};
use crate::error::{AppError, AppResult};
use crate::model::{Character, Shot, SourceKind};
use crate::services::ollama::{Ollama, Prompt};

/// Below this the whole source goes straight to the scriptwriter; above it, it is summarised first.
const MAX_DIRECT_CHARS: usize = 9000;
const CHUNK_CHARS: usize = 6000;
/// Each one costs a set of portraits; shots still show at most 3 (`MAX_IN_SHOT`) to keep faces reliable.
const MAX_CHARACTERS: usize = 6;
const MAX_IN_SHOT: usize = 3;

pub struct Draft {
    pub title: String,
    pub style: String,
    pub characters: Vec<Character>,
    pub shots: Vec<Shot>,
}

pub struct Request<'a> {
    pub source: &'a str,
    pub kind: SourceKind,
    pub language: &'a str,
    pub shot_count: u32,
    pub model: &'a str,
}

/// `progress(done, total, message)` is called between LLM calls.
pub async fn generate(llm: &Ollama, req: &Request<'_>, cancel: &CancellationToken, progress: impl Fn(u32, u32, &str)) -> AppResult<Draft> {
    let source = match req.kind {
        SourceKind::Pdf if req.source.chars().count() > MAX_DIRECT_CHARS => {
            let chunks = chunk_text(req.source, CHUNK_CHARS);
            let total = u32::try_from(chunks.len() + 1).unwrap_or(u32::MAX);
            let mut notes = Vec::with_capacity(chunks.len());
            for (i, chunk) in (0u32..).zip(&chunks) {
                progress(i, total, &format!("Tóm tắt phần {}/{}", i + 1, total - 1));
                notes.push(llm.chat(req.model, &Prompt::new(SUMMARY_PROMPT, chunk), cancel).await?);
            }
            progress(total - 1, total, "Viết storyboard");
            notes.join("\n")
        }
        _ => {
            progress(0, 1, "Viết storyboard");
            req.source.to_owned()
        }
    };
    let user = match req.kind {
        SourceKind::Pdf => format!("Source material:\n<source>\n{source}\n</source>"),
        SourceKind::Prompt => format!("Brief (use well-known facts only, keep it cautious): {source}"),
    };
    let system = SYSTEM_PROMPT.replace("{n}", &req.shot_count.to_string()).replace("{language}", language_name(req.language));
    let schema = schema();
    let raw = llm.chat_json(req.model, &Prompt { schema: Some(&schema), ..Prompt::new(&system, &user) }, cancel).await?;
    normalize(&raw)
}

/// LLM output is untrusted: clamp lengths, de-duplicate ids, drop references to unknown characters.
pub fn normalize(raw: &Value) -> AppResult<Draft> {
    let mut ids = HashSet::new();
    let characters: Vec<Character> = arr(&raw["characters"])
        .filter_map(|c| {
            let name = clip(str_of(&c["name"]), 60);
            let mut id = slug(if str_of(&c["id"]).is_empty() { &name } else { str_of(&c["id"]) });
            while !ids.insert(id.clone()) {
                id.push('2');
            }
            (!name.is_empty()).then(|| Character {
                id,
                name,
                description: clip(str_of(&c["description"]), 600),
                takes: vec![],
                selected_take: None,
            })
        })
        .take(MAX_CHARACTERS)
        .collect();

    let shots: Vec<Shot> = arr(&raw["shots"])
        .filter(|s| !str_of(&s["prompt"]).trim().is_empty())
        .enumerate()
        .map(|(i, s)| {
            let mut seen = HashSet::new();
            let present: Vec<String> = arr(&s["characters"])
                .filter_map(|c| c.as_str().map(slug))
                .filter(|id| characters.iter().any(|k| &k.id == id) && seen.insert(id.clone()))
                .take(MAX_IN_SHOT)
                .collect();
            Shot {
                id: format!("S{:02}", i + 1),
                title: clip(str_of(&s["title"]), 80),
                characters: present,
                framing: clip(str_of(&s["framing"]), 80),
                prompt: clip(str_of(&s["prompt"]), 1200),
                narration: clip(str_of(&s["narration"]), 600),
                motion: clip(str_of(&s["motion"]), 120),
                takes: vec![],
                selected_take: None,
            }
        })
        .collect();

    if shots.is_empty() {
        return Err(AppError::service("Ollama", "Storyboard không có shot nào hợp lệ. Thử lại hoặc đổi model."));
    }
    Ok(Draft { title: clip(str_of(&raw["title"]), 90), style: clip(str_of(&raw["style"]), 400), characters, shots })
}

fn arr(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn clip(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max { s } else { s.chars().take(max - 1).collect::<String>() + "…" }
}

fn language_name(code: &str) -> &str {
    match code {
        "vi" => "Vietnamese (tiếng Việt)",
        "en" => "English",
        other => other,
    }
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "style": { "type": "string" },
            "characters": { "type": "array", "items": { "type": "object", "properties": {
                "id": { "type": "string" }, "name": { "type": "string" }, "description": { "type": "string" }
            }, "required": ["id", "name", "description"] } },
            "shots": { "type": "array", "items": { "type": "object", "properties": {
                "title": { "type": "string" },
                "characters": { "type": "array", "items": { "type": "string" } },
                "framing": { "type": "string" },
                "prompt": { "type": "string" },
                "narration": { "type": "string" },
                "motion": { "type": "string" }
            }, "required": ["title", "characters", "framing", "prompt", "narration"] } }
        },
        "required": ["title", "style", "characters", "shots"]
    })
}

const SUMMARY_PROMPT: &str = "Summarise this part of a story for a storyboard artist in English: key events in order, \
settings, and any physical description of characters (age, hair, clothes). Bullets only.";

const SYSTEM_PROMPT: &str = r#"You are a storyboard artist and screenwriter adapting source material into a short illustrated video.

Output a storyboard with:
- "title": a short title in {language}.
- "style": ONE visual style sentence in English used for every image (medium, palette, era, mood). Prefer illustrated styles
  (watercolor, gouache, anime, graphic novel) over photorealism.
- "characters": every named character who appears in more than one shot, and every character the brief describes
  (max 6). id = short lowercase ascii slug. "description" in ENGLISH: age, gender, ethnicity (or species), build,
  hair/fur (style + color), face, and ONE fixed outfit with colors. Be specific: this exact description is reused for
  every image, so the character must look the same everywhere. Translate figurative descriptions from the source into
  literal features ("head like a leopard, tiger whiskers" becomes "broad square face, bristly spreading black beard"):
  the image model draws comparisons literally. Name animals only for characters that really are animals.
- "shots": exactly {n} shots in story order. For each:
  - "title": a few words in {language}.
  - "characters": ids of ALL recurring characters visible in the frame (0-2 is best, never more than 3).
    A recurring character that is visible MUST be listed. If a scene needs more than 3, show the rest only as small
    generic silhouettes in the background. If a character appears in a different form (transformed, as an animal
    mount), do not list it and describe that form in the prompt instead.
  - "framing": camera framing in English (e.g. "wide establishing shot", "medium two-shot", "close-up").
  - "prompt": ENGLISH visual description of the frame: setting, time of day, weather, what the characters are doing,
    their positions in the frame, poses, expressions, lighting. Refer to listed characters by name and do not describe
    their looks again. Never name a recurring character that is not listed in "characters".
  - "narration": the voice-over line for this shot in {language}, 1-2 natural spoken sentences, in your own words
    (summarise or paraphrase; never copy long passages from the source).
  - "motion": one camera move: "push-in", "pull-out", "pan left", "pan right", "tilt up", "tilt down", "static" or
    "shake", optionally with "slow" or "fast" (e.g. "slow push-in", "fast pan right", "shake" for impacts).
Composition: never use the same framing twice in a row; mix wide, medium, close-up, over-the-shoulder, low-angle and
high-angle views. Vary where the main character is (left third, right third, foreground, small in a wide view) and what
they are doing (walking, running, kneeling, reaching, fighting, looking up), not just standing in the centre.
Stay faithful to the source. No text, logos or captions inside the images."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_cleans_untrusted_llm_output() {
        let raw = json!({
            "title": "Rừng Na Uy",
            "style": "Watercolor",
            "characters": [
                { "id": "Toru", "name": "Toru", "description": "a student" },
                { "id": "toru", "name": "Toru twin", "description": "duplicate id" },
                { "id": "", "name": "", "description": "nameless, dropped" }
            ],
            "shots": [
                { "title": "A", "characters": ["toru", "ghost", "toru"], "framing": "wide", "prompt": "meadow", "narration": "n" },
                { "title": "empty prompt is dropped", "characters": [], "framing": "", "prompt": "  ", "narration": "" },
                { "title": "B", "characters": ["toru2"], "framing": "close", "prompt": "rain", "narration": "n" }
            ]
        });
        let d = normalize(&raw).unwrap();
        assert_eq!(d.characters.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["toru", "toru2"]);
        assert_eq!(d.shots.len(), 2);
        assert_eq!(d.shots[0].characters, ["toru"]);
        assert_eq!((d.shots[1].id.as_str(), d.shots[1].characters.as_slice()), ("S02", ["toru2".to_owned()].as_slice()));
    }

    #[test]
    fn keeps_up_to_six_characters_but_three_per_shot() {
        let chars: Vec<Value> = (0..8).map(|i| json!({ "id": format!("c{i}"), "name": format!("C{i}"), "description": "" })).collect();
        let raw = json!({ "characters": chars, "shots": [{ "characters": ["c0", "c1", "c2", "c3"], "prompt": "p" }] });
        let d = normalize(&raw).unwrap();
        assert_eq!(d.characters.len(), MAX_CHARACTERS);
        assert_eq!(d.shots[0].characters.len(), MAX_IN_SHOT);
    }

    #[test]
    fn normalize_fails_without_shots() {
        assert!(normalize(&json!({ "title": "x", "shots": [] })).is_err());
        assert!(normalize(&json!("not an object")).is_err());
    }
}
