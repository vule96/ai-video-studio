//! Pure parts of the render stage: camera moves, ffmpeg arguments, subtitles, loudness and music mix, WAV files.
//! The job in `jobs.rs` does the I/O.

use std::hash::{DefaultHasher, Hash, Hasher};

pub const FPS: u32 = 30;
/// VieNeu-TTS v3 Turbo's native rate; asking for it avoids resampling.
pub const SAMPLE_RATE: u32 = 48_000;
/// Silence before and after the narration, so a cut never clips a word.
pub const LEAD: f64 = 0.4;
const TAIL: f64 = 0.5;
/// A shot without narration still stays on screen long enough to be seen.
const MIN_CLIP: f64 = 3.0;
const FADE: f64 = 0.4;
/// Final loudness: what YouTube, TikTok and Facebook normalise to, so the video is not quieter than its neighbours.
const TARGET_LUFS: i32 = -14;
/// Music sits this loud before ducking: about 15 dB under the narration.
const MUSIC_LUFS: i32 = -30;
/// Longest subtitle on screen at once: two lines of about 42 characters.
const MAX_CUE_CHARS: usize = 84;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Camera {
    PushIn,
    PullOut,
    PanLeft,
    PanRight,
    TiltUp,
    TiltDown,
    Hold,
    Shake,
}

/// A camera move and how far it travels: 0.6 slow, 1 normal, 1.6 fast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub camera: Camera,
    pub strength: f64,
}

/// Reads the storyboard's free-text hint ("slow push-in", "fast pan left", "shake"). Unknown hints get a push-in.
pub fn motion(hint: &str) -> Motion {
    let m = hint.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| m.contains(w));
    let camera = if has(&["static", "still", "hold", "locked"]) {
        Camera::Hold
    } else if has(&["shake", "shaky", "handheld", "rumble", "tremble", "impact"]) {
        Camera::Shake
    } else if has(&["pull", "zoom out", "dolly out", "widen"]) {
        Camera::PullOut
    } else if has(&["tilt up", "pan up", "crane up", "rise", "upward"]) {
        Camera::TiltUp
    } else if has(&["tilt down", "pan down", "crane down", "downward"]) {
        Camera::TiltDown
    } else if has(&["left"]) {
        Camera::PanLeft
    } else if has(&["right"]) {
        Camera::PanRight
    } else {
        Camera::PushIn
    };
    let strength = if has(&["fast", "quick", "rapid", "dramatic", "whip"]) {
        1.6
    } else if has(&["slow", "gentle", "subtle", "slight"]) {
        0.6
    } else {
        1.0
    };
    Motion { camera, strength }
}

/// Frames for a shot whose narration lasts `audio_secs`. An AI clip (`video_frames`) always plays to its end; when the
/// narration is longer, its last frame holds.
pub fn clip_frames(audio_secs: f64, video_frames: Option<u32>) -> u32 {
    let secs = (LEAD + audio_secs + TAIL).max(MIN_CLIP);
    // Durations are a few seconds to a few minutes; the cast cannot overflow.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frames = (secs * f64::from(FPS)).round() as u32;
    frames.max(video_frames.unwrap_or(0))
}

pub fn frames_secs(frames: u32) -> f64 {
    f64::from(frames) / f64::from(FPS)
}

/// `zoompan` with an eased 0→1 progress, so moves start and stop softly. Strength scales the distance travelled.
fn zoompan(m: Motion, frames: u32, (w, h): (u32, u32)) -> String {
    let t = format!("((1-cos(PI*on/{}))/2)", frames.saturating_sub(1).max(1));
    let (cx, cy) = ("iw/2-iw/zoom/2".to_owned(), "ih/2-ih/zoom/2".to_owned());
    let zoom = 0.12 * m.strength;
    // Pans and tilts zoom in just enough to have room to travel without showing the image edge.
    let pan = format!("{:.3}", 1.0 + 0.10 * m.strength);
    let (z, x, y) = match m.camera {
        Camera::PushIn => (format!("1+{zoom:.3}*{t}"), cx, cy),
        Camera::PullOut => (format!("{:.3}-{zoom:.3}*{t}", 1.0 + zoom), cx, cy),
        Camera::PanLeft => (pan, format!("(iw-iw/zoom)*(1-{t})"), cy),
        Camera::PanRight => (pan, format!("(iw-iw/zoom)*{t}"), cy),
        Camera::TiltUp => (pan, cx, format!("(ih-ih/zoom)*(1-{t})")),
        Camera::TiltDown => (pan, cx, format!("(ih-ih/zoom)*{t}")),
        Camera::Hold => (format!("1+{:.3}*{t}", 0.03 * m.strength), cx, cy),
        // Two sines at unrelated rates read as a hand-held camera; 1.08 zoom leaves room for the wobble.
        Camera::Shake => {
            let a = 14.0 * m.strength;
            ("1.08".into(), format!("{cx}+{a:.1}*sin(on*0.9)+{:.1}*sin(on*2.3)", a / 2.0), format!("{cy}+{a:.1}*sin(on*1.3+1)"))
        }
    };
    format!("zoompan=z='{z}':x='{x}':y='{y}':d={frames}:s={w}x{h}:fps={FPS}")
}

/// What a shot shows. Paths are relative to the project folder (ffmpeg runs there).
pub enum Picture<'a> {
    /// A still frame with a camera move.
    Still { image: &'a str, motion: Motion },
    /// An AI clip with `frames` frames, each shown for one output frame: a 32 fps clip plays about 6% slower at 30 fps,
    /// which is smoother than dropping frames.
    Video { file: &'a str, frames: u32 },
}

/// One shot: the picture, its narration, fades at both ends.
pub struct Clip<'a> {
    pub picture: Picture<'a>,
    pub audio: Option<&'a str>,
    pub audio_secs: f64,
    pub size: (u32, u32),
    pub out: &'a str,
}

impl Clip<'_> {
    pub fn frames(&self) -> u32 {
        let video = match self.picture {
            Picture::Video { frames, .. } => Some(frames),
            Picture::Still { .. } => None,
        };
        clip_frames(self.audio_secs, video)
    }
}

pub fn clip_args(c: &Clip) -> Vec<String> {
    let frames = c.frames();
    let secs = frames_secs(frames);
    let (w, h) = c.size;
    let (input, picture) = match c.picture {
        Picture::Still { image, motion } => {
            // zoompan rounds its crop window to whole pixels; working at 2x the output size hides the jitter.
            let (bw, bh) = (w * 2, h * 2);
            (image, format!("scale={bw}:{bh}:force_original_aspect_ratio=increase,crop={bw}:{bh},{}", zoompan(motion, frames, c.size)))
        }
        Picture::Video { file, .. } => (
            file,
            format!(
                "setpts=N/({FPS}*TB),scale={w}:{h}:force_original_aspect_ratio=increase:flags=lanczos,crop={w}:{h},\
                 tpad=stop_mode=clone:stop_duration={secs:.3},fps={FPS}"
            ),
        ),
    };
    let video = format!("[0:v]{picture},fade=t=in:d={FADE},fade=t=out:st={:.3}:d={FADE},format=yuv420p,setsar=1[v]", secs - FADE);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let lead_ms = (LEAD * 1000.0) as u32;
    let audio = format!("[1:a]aformat=sample_rates={SAMPLE_RATE}:channel_layouts=stereo,adelay={lead_ms}:all=1,apad,atrim=0:{secs:.3}[a]");

    let mut args: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error", "-i", input].map(String::from).to_vec();
    match c.audio {
        Some(audio) => args.extend(["-i".into(), audio.into()]),
        None => args.extend(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo"].map(String::from)),
    }
    args.extend(
        [
            "-filter_complex",
            &format!("{video};{audio}"),
            "-map",
            "[v]",
            "-map",
            "[a]",
            "-frames:v",
            &frames.to_string(),
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "20",
            "-c:a",
            "aac",
            "-b:a",
            "160k",
            "-t",
            &format!("{secs:.3}"),
            c.out,
        ]
        .map(String::from),
    );
    args
}

/// Text for the image-to-video model: the shot's action, the camera move, the project style.
pub fn clip_prompt(action: &str, camera: &str, style: &str) -> String {
    [action.trim().trim_end_matches('.'), &format!("Camera: {}", camera.trim()), style.trim().trim_end_matches('.')]
        .into_iter()
        .filter(|p| !p.is_empty() && *p != "Camera: ")
        .collect::<Vec<_>>()
        .join(". ")
        + "."
}

/// Input for ffmpeg's concat demuxer; names are relative to the list file.
pub fn concat_list(files: &[String]) -> String {
    files.iter().map(|f| format!("file '{f}'\n")).collect()
}

pub fn concat_args(list: &str, out: &str) -> Vec<String> {
    ["-y", "-hide_banner", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", list, "-c", "copy", out].map(String::from).to_vec()
}

pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Splits one shot's narration into subtitle-sized pieces, timed by their share of the characters.
/// Breaks after a sentence, or after a comma once a piece is half full, or when it would overflow.
pub fn cues(start: f64, secs: f64, text: &str) -> Vec<Cue> {
    let mut pieces: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > MAX_CUE_CHARS {
            pieces.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
        let len = cur.chars().count();
        let sentence_end = word.ends_with(['.', '!', '?', '…']);
        if (sentence_end && len >= 20) || (word.ends_with([',', ';', ':']) && len >= MAX_CUE_CHARS / 2) {
            pieces.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        pieces.push(cur);
    }
    #[allow(clippy::cast_precision_loss)]
    let total = pieces.iter().map(|p| p.chars().count()).sum::<usize>().max(1) as f64;
    let mut t = start;
    pieces
        .into_iter()
        .map(|text| {
            #[allow(clippy::cast_precision_loss)]
            let len = secs * text.chars().count() as f64 / total;
            let cue = Cue { start: t, end: t + len, text };
            t += len;
            cue
        })
        .collect()
}

pub fn srt(cues: &[Cue]) -> String {
    let mut out = String::new();
    for (i, c) in cues.iter().filter(|c| !c.text.trim().is_empty()).enumerate() {
        out.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, timestamp(c.start), timestamp(c.end), c.text.trim()));
    }
    out
}

/// Last step: loudness to `TARGET_LUFS`, optional music ducked under the narration, optional burned-in subtitles.
/// Paths are relative to the project folder; `secs` is the video length.
pub struct Finish<'a> {
    pub joined: &'a str,
    pub music: Option<&'a str>,
    pub subtitles: Option<&'a str>,
    pub portrait: bool,
    pub secs: f64,
    pub out: &'a str,
}

pub fn finish_args(f: &Finish) -> Vec<String> {
    let mut args: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error", "-i", f.joined].map(String::from).to_vec();
    let norm = format!("loudnorm=I={TARGET_LUFS}:TP=-1.5:LRA=11,aresample={SAMPLE_RATE}");
    let mut graph = match f.music {
        Some(music) => {
            args.extend(["-stream_loop", "-1", "-i", music].map(String::from));
            let fade_out = (f.secs - 2.5).max(0.0);
            // The narration is the sidechain key: music drops while someone speaks and comes back in the gaps.
            format!(
                "[1:a]aformat=sample_rates={SAMPLE_RATE}:channel_layouts=stereo,atrim=0:{secs:.3},\
                 loudnorm=I={MUSIC_LUFS}:TP=-2,aresample={SAMPLE_RATE},afade=t=in:d=1,afade=t=out:st={fade_out:.3}:d=2.5[m];\
                 [0:a]asplit=2[voice][key];[m][key]sidechaincompress=threshold=0.02:ratio=6:attack=15:release=350[duck];\
                 [voice][duck]amix=inputs=2:duration=first:normalize=0,{norm}[a]",
                secs = f.secs
            )
        }
        None => format!("[0:a]{norm}[a]"),
    };
    let video_codec: &[&str] = match f.subtitles {
        Some(srt) => {
            // Sizes are in the subtitle's 384x288 script units, scaled by height; portrait needs smaller numbers for
            // the same pixel size, and a higher margin to clear TikTok/Reels buttons.
            let (size, margin) = if f.portrait { (9, 70) } else { (16, 18) };
            graph.push_str(&format!(
                ";[0:v]subtitles=filename={srt}:force_style='FontName=Arial,FontSize={size},PrimaryColour=&H00FFFFFF,\
                 OutlineColour=&H00000000,BorderStyle=1,Outline=2,Shadow=0,Alignment=2,MarginV={margin}'[v]"
            ));
            &["-map", "[v]", "-c:v", "libx264", "-preset", "veryfast", "-crf", "20"]
        }
        None => &["-map", "0:v", "-c:v", "copy"],
    };
    args.extend(["-filter_complex".to_owned(), graph]);
    args.extend(video_codec.iter().map(|s| (*s).to_owned()));
    args.extend(["-map", "[a]", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", f.out].map(String::from));
    args
}

fn timestamp(secs: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ms = (secs.max(0.0) * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

/// Wraps raw mono s16le samples in a WAV header.
pub fn wav(pcm: &[u8]) -> Vec<u8> {
    let len = u32::try_from(pcm.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// Length of a file written by `wav`.
pub fn wav_secs(file_len: u64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let samples = file_len.saturating_sub(44) as f64 / 2.0;
    samples / f64::from(SAMPLE_RATE)
}

/// Cache key for a narration: same text and voice reuse the WAV from an earlier render.
pub fn audio_key(voice: &str, text: &str) -> String {
    // shortcut: DefaultHasher may change between Rust releases; that only costs one re-synthesis.
    let mut h = DefaultHasher::new();
    (voice, text).hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_motion_hints() {
        let cam = |h: &str| motion(h).camera;
        assert_eq!(cam("slow push-in"), Camera::PushIn);
        assert_eq!(cam("Gentle pan left"), Camera::PanLeft);
        assert_eq!(cam("pan right across the stadium"), Camera::PanRight);
        assert_eq!(cam("slow pull back to reveal the crowd"), Camera::PullOut);
        assert_eq!(cam("static"), Camera::Hold);
        assert_eq!(cam("tilt up to the sky"), Camera::TiltUp);
        assert_eq!(cam("slow pan up"), Camera::TiltUp);
        assert_eq!(cam("camera shake"), Camera::Shake);
        assert_eq!(cam(""), Camera::PushIn);
        assert!((motion("fast zoom in").strength - 1.6).abs() < 1e-9);
        assert!((motion("slow pan left").strength - 0.6).abs() < 1e-9);
        assert!((motion("pan left").strength - 1.0).abs() < 1e-9);
    }

    #[test]
    fn faster_moves_travel_further() {
        let slow = zoompan(motion("slow push-in"), 90, (1920, 1080));
        let fast = zoompan(motion("fast push-in"), 90, (1920, 1080));
        assert!(slow.contains("z='1+0.072*"));
        assert!(fast.contains("z='1+0.192*"));
        assert!(zoompan(motion("shake"), 90, (1920, 1080)).contains("sin(on*0.9)"));
    }

    #[test]
    fn cues_split_long_narration_and_share_the_time() {
        let text = "Năm 1902, Madrid Football Club ra đời. Từ những con phố lát đá, một câu lạc bộ nhỏ bắt đầu hành trình \
                    trở thành biểu tượng của cả một dân tộc, vượt qua bao thăng trầm.";
        let c = cues(10.0, 8.0, text);
        assert!(c.len() >= 3, "{:?}", c.iter().map(|c| &c.text).collect::<Vec<_>>());
        assert!(c.iter().all(|c| c.text.chars().count() <= MAX_CUE_CHARS));
        assert_eq!(c[0].text, "Năm 1902, Madrid Football Club ra đời.");
        assert!((c[0].start - 10.0).abs() < 1e-9 && (c.last().unwrap().end - 18.0).abs() < 1e-9);
        assert!(cues(0.0, 1.0, "  ").is_empty());
    }

    #[test]
    fn finish_mixes_music_and_burns_subtitles_only_when_asked() {
        let plain = Finish { joined: "j.mp4", music: None, subtitles: None, portrait: false, secs: 60.0, out: "o.mp4" };
        let args = finish_args(&plain);
        assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
        assert!(args.iter().any(|a| a.contains("loudnorm=I=-14")));

        let full = Finish { music: Some("music/m.mp3"), subtitles: Some("render/v.srt"), portrait: true, ..plain };
        let args = finish_args(&full);
        let graph = &args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
        assert!(args.windows(4).any(|w| w == ["-stream_loop", "-1", "-i", "music/m.mp3"]));
        assert!(graph.contains("sidechaincompress") && graph.contains("afade=t=out:st=57.500"));
        assert!(graph.contains("subtitles=filename=render/v.srt") && graph.contains("FontSize=9"));
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
    }

    #[test]
    fn clip_lasts_narration_plus_padding() {
        assert_eq!(clip_frames(10.0, None), (10.9 * 30.0_f64).round() as u32);
        assert_eq!(clip_frames(0.0, None), 90);
        // An AI clip plays to its end even under a short line, and holds its last frame under a long one.
        assert_eq!(clip_frames(2.0, Some(161)), 161);
        assert_eq!(clip_frames(10.0, Some(161)), 327);
    }

    #[test]
    fn clip_args_use_silence_without_narration() {
        let clip = Clip {
            picture: Picture::Still { image: "shots/a.png", motion: motion("pan left") },
            audio: None,
            audio_secs: 0.0,
            size: (1920, 1080),
            out: "render/tmp/000.mp4",
        };
        let args = clip_args(&clip);
        assert!(args.iter().any(|a| a.starts_with("anullsrc")));
        let filter = &args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
        assert!(filter.contains("crop=3840:2160"));
        assert!(filter.contains("d=90:s=1920x1080"));
        assert!(filter.contains("fade=t=out:st=2.600"));
        assert_eq!(args.last().unwrap(), "render/tmp/000.mp4");
    }

    #[test]
    fn clip_prompt_joins_action_camera_and_style() {
        assert_eq!(clip_prompt("Toru walks. ", "slow push-in", "Watercolor."), "Toru walks. Camera: slow push-in. Watercolor.");
        assert_eq!(clip_prompt("Toru walks", " ", ""), "Toru walks.");
    }

    #[test]
    fn video_clips_play_frame_by_frame_and_hold_the_end() {
        let clip = Clip {
            picture: Picture::Video { file: "clips/S01.mp4", frames: 161 },
            audio: Some("render/audio/a.wav"),
            audio_secs: 2.0,
            size: (1920, 1080),
            out: "o.mp4",
        };
        let args = clip_args(&clip);
        let filter = &args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
        assert!(args.windows(2).any(|w| w == ["-i", "clips/S01.mp4"]));
        assert!(filter.starts_with("[0:v]setpts=N/(30*TB),scale=1920:1080"));
        assert!(filter.contains("tpad=stop_mode=clone"));
        assert!(!filter.contains("zoompan"));
        assert!(args.windows(2).any(|w| w == ["-frames:v", "161"]));
    }

    #[test]
    fn srt_skips_silent_shots_and_formats_time() {
        let cue = |start, end, text: &str| Cue { start, end, text: text.into() };
        let cues = [cue(0.4, 3.25, "Năm 1902."), cue(4.0, 5.0, "  "), cue(3661.0, 3662.5, "Bernabéu")];
        assert_eq!(srt(&cues), "1\n00:00:00,400 --> 00:00:03,250\nNăm 1902.\n\n2\n01:01:01,000 --> 01:01:02,500\nBernabéu\n\n");
    }

    #[test]
    fn wav_roundtrips_length() {
        let file = wav(&vec![0u8; 96_000]);
        assert_eq!(&file[..4], b"RIFF");
        assert!((wav_secs(file.len() as u64) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn audio_key_depends_on_voice_and_text() {
        assert_eq!(audio_key("Thiện Minh", "a"), audio_key("Thiện Minh", "a"));
        assert_ne!(audio_key("Thiện Minh", "a"), audio_key("Mai Anh", "a"));
        assert_ne!(audio_key("Thiện Minh", "a"), audio_key("Thiện Minh", "b"));
    }
}
