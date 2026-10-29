//! ComfyUI workflow (API format) for FLUX.2 [klein]: text-to-image when there are no
//! reference images, multi-reference edit otherwise. Mirrors ComfyUI's own
//! "Image Edit (Flux.2 Klein 4B)" blueprint, extended to N references.

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
    fn text_to_image_has_no_reference_nodes() {
        let req = ImageRequest { prompt: "p", width: 832, height: 1216, seed: 1, references: &[], filename_prefix: "x" };
        let g = image_graph(&ImageModel::default(), &req);
        assert_eq!(count(&g, "LoadImage") + count(&g, "ReferenceLatent"), 0);
    }
}
