//! Frame review: a vision model compares each frame with the cast and description it was drawn from.

use serde_json::{Value, json};

use crate::model::{Character, Review, Shot, Take};

pub const SYSTEM_PROMPT: &str = "You are a continuity supervisor checking illustrated storyboard frames before a video is \
published. Compare the image with the expected cast and scene and sort what you see into major problems, which make \
the frame unusable, and minor differences, which a viewer would accept. Write each item as one short sentence in \
Vietnamese.";

pub fn user_prompt(shot: &Shot, cast: &[&Character]) -> String {
    let cast_lines: Vec<String> = cast.iter().map(|c| format!("- {}: {}", c.name, c.description)).collect();
    let cast = if cast_lines.is_empty() {
        "No recurring character should be in this frame; background figures are fine.".to_owned()
    } else {
        format!("Each of these must appear exactly once:\n{}", cast_lines.join("\n"))
    };
    format!(
        "Expected cast. {cast}\n\nExpected scene ({}): {}\n\n\
         major = only these: a listed character is missing; a listed character appears twice; a prominent figure that \
         is not listed; a character is the wrong kind of being (a human instead of an animal, a four-legged animal \
         instead of an upright warrior); a character's main outfit or headwear is clearly different (for example a \
         straw hat instead of a crown); large legible text, letters or logos; broken anatomy (extra limbs, merged \
         bodies); the key object or setting of the scene is wrong (a wheeled cart instead of a carried sedan chair). \
         minor = everything else that differs: pose, gesture, expression, camera framing or angle, lighting, colour \
         shades, eye colour, small accessories, a small seal or signature mark, how closely the style matches. \
         Held items only matter when the scene shows them; never report one as missing. \
         Answer as JSON: major = list (empty when the frame is usable), minor = list.",
        shot.framing, shot.prompt
    )
}

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "major": { "type": "array", "items": { "type": "string" } },
            "minor": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["major", "minor"]
    })
}

/// Model output is untrusted: at most five short items per list; a frame is ok when nothing major was found.
pub fn parse(raw: &Value) -> Review {
    let list = |key: &str| -> Vec<String> {
        raw[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|i| i.as_str())
            .map(|i| i.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|i| !i.is_empty())
            .map(|i| if i.chars().count() > 200 { i.chars().take(199).collect::<String>() + "…" } else { i })
            .take(5)
            .collect()
    };
    let issues = list("major");
    Review { ok: issues.is_empty(), issues, notes: list("minor") }
}

/// The take to keep after an automatic fix: the newest new take that passed review. When none passed, the take
/// selected before the fix stays: issue counts from a small vision model say nothing about which failing image
/// looks better, and swapping one flawed frame for another made scenes worse.
pub fn pick_after_fix<'a>(takes: &'a [Take], new_ids: &[String], before: Option<&'a str>) -> Option<&'a str> {
    let passed = takes.iter().rev().find(|t| new_ids.contains(&t.id) && t.review.as_ref().is_some_and(|r| r.ok));
    passed.map(|t| t.id.as_str()).or(before)
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
    fn only_major_issues_flag_a_frame() {
        let r = parse(&json!({ "major": ["Thanh Sư xuất hiện hai lần", "  "], "minor": ["Mắt màu nâu"] }));
        assert!(!r.ok);
        assert_eq!(r.issues, ["Thanh Sư xuất hiện hai lần"]);
        assert_eq!(r.notes, ["Mắt màu nâu"]);
        let minor_only = parse(&json!({ "major": [], "minor": ["Góc máy không phải cận cảnh"] }));
        assert!(minor_only.ok);
        assert!(parse(&json!("garbage")).ok);
    }

    #[test]
    fn fix_keeps_the_old_take_unless_a_new_one_passes() {
        let take = |id: &str, review: Option<(bool, usize)>| Take {
            id: id.into(),
            file: String::new(),
            seed: 0,
            created_at: 0,
            review: review.map(|(ok, n)| Review { ok, issues: vec!["x".into(); n], notes: vec![] }),
        };
        let new = ["b".to_owned(), "c".to_owned(), "d".to_owned()];
        let all_fail = [take("a", Some((false, 2))), take("b", Some((false, 1))), take("c", Some((false, 1))), take("d", Some((false, 3)))];
        assert_eq!(pick_after_fix(&all_fail, &new, Some("a")), Some("a"));
        let two_pass = [take("a", Some((false, 1))), take("b", Some((true, 0))), take("c", Some((false, 1))), take("d", Some((true, 0)))];
        assert_eq!(pick_after_fix(&two_pass, &new, Some("a")), Some("d"));
        // An old take that passed earlier is not a "new" winner; the selection before the fix stays.
        let old_pass = [take("a", Some((true, 0))), take("b", Some((false, 1)))];
        assert_eq!(pick_after_fix(&old_pass, &new, Some("b")), Some("b"));
    }
}
