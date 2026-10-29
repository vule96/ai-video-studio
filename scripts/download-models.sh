#!/usr/bin/env bash
# Downloads the image model for storyboards into ComfyUI's models folder (~12.5 GB, resumable).
#   ./scripts/download-models.sh ~/ComfyUI/models [--video]
# FLUX.2 [klein] 4B distilled FP8 (Apache 2.0) + Qwen3 4B text encoder + FLUX.2 VAE.
# --video adds the AI motion models (~38 GB): Wan 2.2 I2V 14B FP8 + lightx2v 4-step LoRAs (Apache 2.0),
# Real-ESRGAN x2 (BSD-3) for 480p -> 720p, RIFE (MIT) for 16 -> 32 fps.
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

if [ "${2:-}" = "--video" ]; then
  fetch diffusion_models wan2.2_i2v_high_noise_14B_fp8_scaled.safetensors \
    https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/diffusion_models/wan2.2_i2v_high_noise_14B_fp8_scaled.safetensors
  fetch diffusion_models wan2.2_i2v_low_noise_14B_fp8_scaled.safetensors \
    https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/diffusion_models/wan2.2_i2v_low_noise_14B_fp8_scaled.safetensors
  fetch loras wan2.2_i2v_lightx2v_4steps_lora_v1_high_noise.safetensors \
    https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/loras/wan2.2_i2v_lightx2v_4steps_lora_v1_high_noise.safetensors
  fetch loras wan2.2_i2v_lightx2v_4steps_lora_v1_low_noise.safetensors \
    https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/loras/wan2.2_i2v_lightx2v_4steps_lora_v1_low_noise.safetensors
  fetch text_encoders umt5_xxl_fp8_e4m3fn_scaled.safetensors \
    https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/resolve/main/split_files/text_encoders/umt5_xxl_fp8_e4m3fn_scaled.safetensors
  fetch vae wan_2.1_vae.safetensors \
    https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/vae/wan_2.1_vae.safetensors
  fetch upscale_models RealESRGAN_x2plus.pth \
    https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth
  fetch frame_interpolation rife_v4.26.safetensors \
    https://huggingface.co/Comfy-Org/frame_interpolation/resolve/main/frame_interpolation/rife_v4.26.safetensors
fi

echo "Done. Restart ComfyUI so it picks up the new models."
