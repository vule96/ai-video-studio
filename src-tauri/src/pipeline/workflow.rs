//! ComfyUI workflows (API format).
//! Images: FLUX.2 [klein], text-to-image when there are no reference images, multi-reference edit otherwise. Mirrors
//! ComfyUI's own "Image Edit (Flux.2 Klein 4B)" blueprint, extended to N references.
//! Clips: Wan 2.2 14B image-to-video with the lightx2v 4-step LoRAs, RIFE to double the frame rate.

use serde_json::{Map, Value, json};

use crate::settings::ImageModel;

pub struct ImageRequest<'a> {
    pub prompt: &'a str,
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    /// Names returned by `Comfy::upload_image`.
    pub references: &'a [String],
    pub filename_prefix: &'a str,
}

/// Node ids are just a counter; links are `[node_id, output_slot]`.
struct Graph(Map<String, Value>);

impl Graph {
    fn add(&mut self, class_type: &str, inputs: Value) -> Value {
        let id = (self.0.len() + 1).to_string();
        self.0.insert(id.clone(), json!({ "class_type": class_type, "inputs": inputs }));
        json!([id, 0])
    }
}

/// Wan 2.2 files as `scripts/download-models.* -Video` saves them; the health check looks for these names.
pub const WAN_HIGH: &str = "wan2.2_i2v_high_noise_14B_fp8_scaled.safetensors";
pub const WAN_LOW: &str = "wan2.2_i2v_low_noise_14B_fp8_scaled.safetensors";
pub const WAN_LORA_HIGH: &str = "wan2.2_i2v_lightx2v_4steps_lora_v1_high_noise.safetensors";
pub const WAN_LORA_LOW: &str = "wan2.2_i2v_lightx2v_4steps_lora_v1_low_noise.safetensors";
pub const WAN_TEXT_ENCODER: &str = "umt5_xxl_fp8_e4m3fn_scaled.safetensors";
pub const WAN_VAE: &str = "wan_2.1_vae.safetensors";
pub const UPSCALER: &str = "RealESRGAN_x2plus.pth";
pub const RIFE: &str = "rife_v4.26.safetensors";
/// Wan's native rate; RIFE doubles it.
pub const WAN_FPS: u32 = 16;
/// 81 frames at 16 fps, about five seconds: the length the model was trained on.
pub const WAN_FRAMES: u32 = 81;
/// Frames in the saved clip: RIFE puts one new frame between each pair.
pub const CLIP_FRAMES: u32 = 2 * WAN_FRAMES - 1;
/// The usual Wan negative prompt (Chinese, as the model was trained); with CFG 1 it barely matters, but it is cheap.
const WAN_NEGATIVE: &str = "色调艳丽，过曝，静态，细节模糊不清，字幕，风格，作品，画作，画面，静止，整体发灰，最差质量，低质量，JPEG压缩残留，\
丑陋的，残缺的，多余的手指，画得不好的手部，画得不好的脸部，畸形的，毁容的，形态畸形的肢体，手指融合，静止不动的画面，杂乱的背景，三条腿，\
背景人很多，倒着走";

pub struct ClipRequest<'a> {
    pub prompt: &'a str,
    /// Name returned by `Comfy::upload_image`: the frame the clip starts from.
    pub image: &'a str,
    /// Size of the saved clip, e.g. 1280x720.
    pub width: u32,
    pub height: u32,
    /// Draw at full size instead of 2/3 size plus upscaling. On 16 GB the model then spills into RAM: ~3x slower.
    pub hd: bool,
    pub seed: u32,
    pub filename_prefix: &'a str,
}

pub fn clip_graph(req: &ClipRequest<'_>) -> Value {
    let mut g = Graph(Map::new());
    // fp8 matmul on RTX 40/50 cards; older cards fall back to the normal path.
    let unet = |g: &mut Graph, file: &str| g.add("UNETLoader", json!({ "unet_name": file, "weight_dtype": "fp8_e4m3fn_fast" }));
    let high = unet(&mut g, WAN_HIGH);
    let low = unet(&mut g, WAN_LOW);
    let model = |g: &mut Graph, unet: Value, lora: &str| {
        let m = g.add("LoraLoaderModelOnly", json!({ "model": unet, "lora_name": lora, "strength_model": 1.0 }));
        g.add("ModelSamplingSD3", json!({ "model": m, "shift": 5.0 }))
    };
    let high = model(&mut g, high, WAN_LORA_HIGH);
    let low = model(&mut g, low, WAN_LORA_LOW);
    let clip = g.add("CLIPLoader", json!({ "clip_name": WAN_TEXT_ENCODER, "type": "wan", "device": "default" }));
    let pos = g.add("CLIPTextEncode", json!({ "clip": clip, "text": req.prompt }));
    let neg = g.add("CLIPTextEncode", json!({ "clip": clip, "text": WAN_NEGATIVE }));
    let vae = g.add("VAELoader", json!({ "vae_name": WAN_VAE }));
    let image = g.add("LoadImage", json!({ "image": req.image }));
    // Wan needs multiples of 16; 2/3 of 1280x720 is 853x480, rounded to 848x480 (832 when the result is not divisible).
    let (w, h) = if req.hd { (req.width, req.height) } else { (round16(req.width * 2 / 3), round16(req.height * 2 / 3)) };
    let i2v = g.add(
        "WanImageToVideo",
        json!({ "positive": pos, "negative": neg, "vae": vae, "start_image": image, "width": w, "height": h, "length": WAN_FRAMES, "batch_size": 1 }),
    );
    let cond = |slot: u8| json!([i2v[0], slot]);
    // lightx2v splits 4 steps: the high-noise model lays out the motion, the low-noise model adds detail.
    let sampler = |g: &mut Graph, model: Value, latent: Value, first: bool| {
        g.add(
            "KSamplerAdvanced",
            json!({
                "model": model, "add_noise": if first { "enable" } else { "disable" }, "noise_seed": req.seed, "steps": 4, "cfg": 1.0,
                "sampler_name": "euler", "scheduler": "simple", "positive": cond(0), "negative": cond(1), "latent_image": latent,
                "start_at_step": if first { 0 } else { 2 }, "end_at_step": if first { 2 } else { 10000 },
                "return_with_leftover_noise": if first { "enable" } else { "disable" }
            }),
        )
    };
    let latent = sampler(&mut g, high, cond(2), true);
    let latent = sampler(&mut g, low, latent, false);
    // Tiled decode avoids the VRAM peak at the end of a run, when the desktop stuttered most.
    let mut frames = g.add(
        "VAEDecodeTiled",
        json!({ "samples": latent, "vae": vae, "tile_size": 512, "overlap": 64, "temporal_size": 32, "temporal_overlap": 8 }),
    );
    if !req.hd {
        let upscaler = g.add("UpscaleModelLoader", json!({ "model_name": UPSCALER }));
        let up = g.add("ImageUpscaleWithModel", json!({ "upscale_model": upscaler, "image": frames }));
        frames = g.add(
            "ImageScale",
            json!({ "image": up, "upscale_method": "lanczos", "width": req.width, "height": req.height, "crop": "center" }),
        );
    }
    let rife = g.add("FrameInterpolationModelLoader", json!({ "model_name": RIFE }));
    let smooth = g.add("FrameInterpolate", json!({ "interp_model": rife, "images": frames, "multiplier": 2 }));
    let video = g.add("CreateVideo", json!({ "images": smooth, "fps": 2 * WAN_FPS, "codec": "auto" }));
    g.add("SaveVideo", json!({ "video": video, "filename_prefix": req.filename_prefix, "format": "auto" }));
    Value::Object(g.0)
}

const fn round16(n: u32) -> u32 {
    n / 16 * 16
}

pub fn image_graph(model: &ImageModel, req: &ImageRequest<'_>) -> Value {
    let mut g = Graph(Map::new());
    let unet = g.add("UNETLoader", json!({ "unet_name": model.unet, "weight_dtype": "default" }));
    let clip = g.add("CLIPLoader", json!({ "clip_name": model.text_encoder, "type": "flux2", "device": "default" }));
    let vae = g.add("VAELoader", json!({ "vae_name": model.vae }));
    let mut pos = g.add("CLIPTextEncode", json!({ "clip": clip, "text": req.prompt }));
    let mut neg = g.add("CLIPTextEncode", json!({ "clip": clip, "text": "" }));

    // Each reference: load → ~1 MP → encode → attach to both conditionings.
    for name in req.references {
        let img = g.add("LoadImage", json!({ "image": name }));
        let scaled = g.add(
            "ImageScaleToTotalPixels",
            json!({ "image": img, "upscale_method": "nearest-exact", "megapixels": 1.0, "resolution_steps": 1 }),
        );
        let latent = g.add("VAEEncode", json!({ "pixels": scaled, "vae": vae }));
        pos = g.add("ReferenceLatent", json!({ "conditioning": pos, "latent": latent }));
        neg = g.add("ReferenceLatent", json!({ "conditioning": neg, "latent": latent }));
    }

    let guider = g.add("CFGGuider", json!({ "model": unet, "positive": pos, "negative": neg, "cfg": model.cfg }));
    let sampler = g.add("KSamplerSelect", json!({ "sampler_name": "euler" }));
    let sigmas = g.add("Flux2Scheduler", json!({ "steps": model.steps, "width": req.width, "height": req.height }));
    let noise = g.add("RandomNoise", json!({ "noise_seed": req.seed }));
    let latent = g.add("EmptyFlux2LatentImage", json!({ "width": req.width, "height": req.height, "batch_size": 1 }));
    let out = g.add(
        "SamplerCustomAdvanced",
        json!({ "noise": noise, "guider": guider, "sampler": sampler, "sigmas": sigmas, "latent_image": latent }),
    );
    let decoded = g.add("VAEDecode", json!({ "samples": out, "vae": vae }));
    g.add("SaveImage", json!({ "images": decoded, "filename_prefix": req.filename_prefix }));
    Value::Object(g.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(g: &Value, class: &str) -> usize {
        g.as_object().unwrap().values().filter(|n| n["class_type"] == class).count()
    }

    #[test]
    fn every_link_points_to_an_existing_node() {
        let refs = vec!["a.png".to_owned(), "b.png".to_owned()];
        let req = ImageRequest { prompt: "p", width: 1344, height: 768, seed: 7, references: &refs, filename_prefix: "x" };
        let g = image_graph(&ImageModel::default(), &req);
        let nodes = g.as_object().unwrap();
        for node in nodes.values() {
            for v in node["inputs"].as_object().unwrap().values() {
                if let Some([id, _]) = v.as_array().map(Vec::as_slice) {
                    assert!(nodes.contains_key(id.as_str().unwrap()), "dangling link {v}");
                }
            }
        }
        assert_eq!(count(&g, "LoadImage"), 2);
        assert_eq!(count(&g, "ReferenceLatent"), 4);
        assert_eq!(count(&g, "SaveImage"), 1);
    }

    #[test]
    fn clip_graph_upscales_unless_hd() {
        let req = ClipRequest { prompt: "walks", image: "a.png", width: 1280, height: 720, hd: false, seed: 1, filename_prefix: "x" };
        let fast = clip_graph(&req);
        let i2v = fast.as_object().unwrap().values().find(|n| n["class_type"] == "WanImageToVideo").unwrap();
        assert_eq!((i2v["inputs"]["width"].as_u64(), i2v["inputs"]["height"].as_u64()), (Some(848), Some(480)));
        assert_eq!(count(&fast, "ImageUpscaleWithModel"), 1);
        assert_eq!(count(&fast, "FrameInterpolate"), 1);
        assert_eq!(count(&fast, "SaveVideo"), 1);

        let hd = clip_graph(&ClipRequest { hd: true, width: 720, height: 1280, ..req });
        let i2v = hd.as_object().unwrap().values().find(|n| n["class_type"] == "WanImageToVideo").unwrap();
        assert_eq!((i2v["inputs"]["width"].as_u64(), i2v["inputs"]["height"].as_u64()), (Some(720), Some(1280)));
        assert_eq!(count(&hd, "ImageUpscaleWithModel"), 0);
        assert_links_resolve(&hd);
        assert_links_resolve(&fast);
    }

    fn assert_links_resolve(g: &Value) {
        let nodes = g.as_object().unwrap();
        for node in nodes.values() {
            for v in node["inputs"].as_object().unwrap().values() {
                if let Some([id, _]) = v.as_array().map(Vec::as_slice) {
                    assert!(nodes.contains_key(id.as_str().unwrap()), "dangling link {v}");
                }
            }
        }
    }

    #[test]
    fn text_to_image_has_no_reference_nodes() {
        let req = ImageRequest { prompt: "p", width: 832, height: 1216, seed: 1, references: &[], filename_prefix: "x" };
        let g = image_graph(&ImageModel::default(), &req);
        assert_eq!(count(&g, "LoadImage") + count(&g, "ReferenceLatent"), 0);
    }
}
