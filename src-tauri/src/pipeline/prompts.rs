//! Image prompts. Character looks live in one place (the character description) and are
//! repeated in every shot, which together with the reference image keeps faces consistent.

use crate::model::{Character, Shot};

pub const CHARACTER_SIZE: (u32, u32) = (832, 1216);

/// Face traits the description rarely pins down. Distilled 4-step models give near-identical
/// images for different seeds, so each candidate gets its own "casting" to offer a real choice.
const CASTING: [&str; 6] = [
    "oval face, soft balanced features",
    "angular face, defined jawline, sharper features",
    "round face, gentle wide-set eyes, fuller cheeks",
    "heart-shaped face, high cheekbones, narrow chin",
    "long face, straight nose, thin eyebrows",
    "square face, thick eyebrows, strong brow",
];

/// `variant` picks the casting; pass the running take count so every new batch differs too.
pub fn character_prompt(style: &str, c: &Character, variant: usize) -> String {
    let casting = CASTING[variant % CASTING.len()];
    format!(
        "{style}. Character design reference: a single full-body portrait of {}, {}. \
         Facial structure: {casting}. \
         Standing, facing the viewer, relaxed neutral pose, whole body visible from head to shoes, \
         plain light warm-grey background, soft even lighting. Only one person. No text.",
        c.name, c.description
    )
}

/// `present` are the shot's characters in reference-image order.
pub fn shot_prompt(style: &str, shot: &Shot, present: &[&Character]) -> String {
    let mut parts = vec![format!("{style}.")];
    if present.is_empty() {
        parts.push("No people in the frame.".into());
    } else {
        let refs: Vec<String> = present.iter().enumerate().map(|(i, c)| format!("image {} is {}", i + 1, c.name)).collect();
        parts.push(format!(
            "Reference images: {}. Keep each character's face, hairstyle, body and outfit exactly as in their \
             reference image, but redraw them naturally inside the new scene (do not copy the reference background or pose).",
            refs.join("; ")
        ));
        // Repeating the looks helps when a character is small in a wide shot.
        parts.extend(present.iter().map(|c| format!("{}: {}.", c.name, c.description)));
        // FLUX tends to add a second copy of a referenced character or extra people; state the cast explicitly.
        let names: Vec<&str> = present.iter().map(|c| c.name.as_str()).collect();
        parts.push(format!(
            "Main characters in the frame: exactly {}: {}, each appearing only once. Any other figures are small and indistinct.",
            present.len(),
            names.join(", ")
        ));
    }
    parts.push(format!("{}. {}", capitalize(&shot.framing), shot.prompt));
    parts.push("Cinematic composition, no text, no captions, no watermark.".into());
    parts.join(" ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|f| f.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(name: &str) -> Character {
        Character { id: name.to_lowercase(), name: name.into(), description: format!("{name} looks"), takes: vec![], selected_take: None }
    }

    fn shot() -> Shot {
        Shot {
            id: "S01".into(),
            title: String::new(),
            characters: vec![],
            framing: "medium two-shot".into(),
            prompt: "In a meadow.".into(),
            narration: String::new(),
            motion: String::new(),
            takes: vec![],
            selected_take: None,
            review: None,
        }
    }

    #[test]
    fn numbers_references_in_order() {
        let (a, b) = (character("Toru"), character("Naoko"));
        let p = shot_prompt("Watercolor", &shot(), &[&a, &b]);
        assert!(p.contains("image 1 is Toru; image 2 is Naoko"));
        assert!(p.contains("Medium two-shot. In a meadow."));
        assert!(p.contains("exactly 2: Toru, Naoko, each appearing only once"));
    }

    #[test]
    fn candidates_get_different_castings() {
        let c = character("Toru");
        let prompts: std::collections::HashSet<_> = (0..3).map(|i| character_prompt("Watercolor", &c, i)).collect();
        assert_eq!(prompts.len(), 3);
        assert_eq!(character_prompt("W", &c, 0), character_prompt("W", &c, CASTING.len()));
    }

    #[test]
    fn empty_scene_says_no_people() {
        assert!(shot_prompt("Watercolor", &shot(), &[]).contains("No people in the frame."));
    }
}
