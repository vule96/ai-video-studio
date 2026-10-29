#!/usr/bin/env bash
# Downloads the image model for storyboards into ComfyUI's models folder (~12.5 GB, resumable).
#   ./scripts/download-models.sh ~/ComfyUI/models
# FLUX.2 [klein] 4B distilled FP8 (Apache 2.0) + Qwen3 4B text encoder + FLUX.2 VAE.
set -euo pipefail

models="${1:?Usage: $0 <ComfyUI models folder>}"
[ -d "$models" ] || { echo "Folder not found: $models" >&2; exit 1; }

fetch() { # <subfolder> <file> <url>
  local out="$models/$1/$2"
  mkdir -p "$models/$1"
  if [ -f "$out" ]; then echo "Already there: $1/$2"; return; fi
  echo "==> $1/$2"
  curl -L --fail -C - -o "$out.part" "$3"
  mv "$out.part" "$out"
}

fetch diffusion_models flux-2-klein-4b-fp8.safetensors \
  https://huggingface.co/black-forest-labs/FLUX.2-klein-4b-fp8/resolve/main/flux-2-klein-4b-fp8.safetensors
fetch text_encoders qwen_3_4b.safetensors \
  https://huggingface.co/Comfy-Org/z_image_turbo/resolve/main/split_files/text_encoders/qwen_3_4b.safetensors
fetch vae flux2-vae.safetensors \
  https://huggingface.co/Comfy-Org/flux2-dev/resolve/main/split_files/vae/flux2-vae.safetensors

echo "Done. Restart ComfyUI so it picks up the new models."
