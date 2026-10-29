//! Frame review: a vision model compares each frame with the cast and description it was drawn from.

use serde_json::{Value, json};

use crate::model::{Character, Review, Shot};

pub const SYSTEM_PROMPT: &str = "You are a strict continuity supervisor checking illustrated storyboard frames before a video \
is published. Compare the image with the expected cast and scene. Report only problems you can clearly see; do not \
nitpick style. Write each issue as one short sentence in Vietnamese.";

pub fn user_prompt(shot: &Shot, cast: &[&Character]) -> String {
    let cast_lines: Vec<String> = cast.iter().map(|c| format!("- {}: {}", c.name, c.description)).collect();
    let cast = if cast_lines.is_empty() {
        "No recurring character should be in this frame; background figures are fine.".to_owned()
    } else {
        format!("Each of these must appear exactly once:\n{}", cast_lines.join("\n"))
    };
    format!(
        "Expected cast. {cast}\n\nExpected scene ({}): {}\n\n\
         Check for: a listed character missing; a listed character drawn twice; an extra prominent figure that is not \
         listed; a character whose species, clothes or colors clearly differ from the description; visible text, \
         letters, logos or watermarks; broken anatomy (extra limbs, merged bodies); a scene that clearly does not match. \
         Answer as JSON: ok = true when there is no clear problem, issues = list of problems (empty when ok).",
        shot.framing, shot.prompt
    )
}

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": { "ok": { "type": "boolean" }, "issues": { "type": "array", "items": { "type": "string" } } },
        "required": ["ok", "issues"]
    })
}

/// Model output is untrusted: keep at most five short issues; "ok" with issues listed counts as not ok.
pub fn parse(raw: &Value, take_id: &str) -> Review {
    let issues: Vec<String> = raw["issues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| i.as_str())
        .map(|i| i.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|i| !i.is_empty())
        .map(|i| if i.chars().count() > 200 { i.chars().take(199).collect::<String>() + "…" } else { i })
        .take(5)
        .collect();
    Review { take_id: take_id.to_owned(), ok: issues.is_empty() && raw["ok"].as_bool().unwrap_or(true), issues }
}

/// Standard base64 for Ollama's `images` field.
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ABC[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        for (input, want) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(input.as_bytes()), want);
        }
    }

    #[test]
    fn parse_distrusts_ok_with_issues() {
        let r = parse(&json!({ "ok": true, "issues": ["Thanh Sư xuất hiện hai lần", "  "] }), "S09-1");
        assert!(!r.ok);
        assert_eq!(r.issues, ["Thanh Sư xuất hiện hai lần"]);
        assert!(parse(&json!({ "ok": true, "issues": [] }), "t").ok);
        assert!(!parse(&json!({ "ok": false, "issues": [] }), "t").ok);
    }
}
