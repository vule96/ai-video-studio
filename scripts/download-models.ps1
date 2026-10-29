# Tải model vẽ hình vào thư mục models của ComfyUI.
# Tổng ~12.5GB. Tải lại được nếu đứt mạng (curl -C -).
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\download-models.ps1 -ComfyModels "D:\ComfyUI_windows_portable\ComfyUI\models"
#
# Model: FLUX.2 [klein] 4B distilled FP8 (Apache 2.0) + text encoder Qwen3 4B + FLUX.2 VAE
#
# Thêm -Video để tải model chuyển động AI (~38GB): Wan 2.2 I2V 14B FP8 + LoRA lightx2v 4 bước (Apache 2.0),
# Real-ESRGAN x2 (BSD-3) để phóng 480p lên 720p, RIFE (MIT) để nâng 16 lên 32 fps.

param(
  [Parameter(Mandatory = $true)][string]$ComfyModels,
  [switch]$Video
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path $ComfyModels)) { throw "Không thấy thư mục: $ComfyModels" }

$files = @(
  @{ dir = "diffusion_models"; name = "flux-2-klein-4b-fp8.safetensors"; size = "4.1GB";
     url = "https://huggingface.co/black-forest-labs/FLUX.2-klein-4b-fp8/resolve/main/flux-2-klein-4b-fp8.safetensors" },
  @{ dir = "text_encoders"; name = "qwen_3_4b.safetensors"; size = "~8GB";
     url = "https://huggingface.co/Comfy-Org/z_image_turbo/resolve/main/split_files/text_encoders/qwen_3_4b.safetensors" },
  @{ dir = "vae"; name = "flux2-vae.safetensors"; size = "~0.3GB";
     url = "https://huggingface.co/Comfy-Org/flux2-dev/resolve/main/split_files/vae/flux2-vae.safetensors" }
)

if ($Video) {
  $files += @(
  @{ dir = "diffusion_models"; name = "wan2.2_i2v_high_noise_14B_fp8_scaled.safetensors"; size = "14.3GB";
     url = "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/diffusion_models/wan2.2_i2v_high_noise_14B_fp8_scaled.safetensors" },
  @{ dir = "diffusion_models"; name = "wan2.2_i2v_low_noise_14B_fp8_scaled.safetensors"; size = "14.3GB";
     url = "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/diffusion_models/wan2.2_i2v_low_noise_14B_fp8_scaled.safetensors" },
  @{ dir = "loras"; name = "wan2.2_i2v_lightx2v_4steps_lora_v1_high_noise.safetensors"; size = "1.2GB";
     url = "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/loras/wan2.2_i2v_lightx2v_4steps_lora_v1_high_noise.safetensors" },
  @{ dir = "loras"; name = "wan2.2_i2v_lightx2v_4steps_lora_v1_low_noise.safetensors"; size = "1.2GB";
     url = "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/loras/wan2.2_i2v_lightx2v_4steps_lora_v1_low_noise.safetensors" },
  @{ dir = "text_encoders"; name = "umt5_xxl_fp8_e4m3fn_scaled.safetensors"; size = "6.7GB";
     url = "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/resolve/main/split_files/text_encoders/umt5_xxl_fp8_e4m3fn_scaled.safetensors" },
  @{ dir = "vae"; name = "wan_2.1_vae.safetensors"; size = "0.25GB";
     url = "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/vae/wan_2.1_vae.safetensors" },
  @{ dir = "upscale_models"; name = "RealESRGAN_x2plus.pth"; size = "67MB";
     url = "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth" },
  @{ dir = "frame_interpolation"; name = "rife_v4.26.safetensors"; size = "23MB";
     url = "https://huggingface.co/Comfy-Org/frame_interpolation/resolve/main/frame_interpolation/rife_v4.26.safetensors" }
  )
}

foreach ($f in $files) {
  $dir = Join-Path $ComfyModels $f.dir
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  $out = Join-Path $dir $f.name
  if (Test-Path $out) { Write-Host "Đã có: $($f.dir)\$($f.name)"; continue }
  Write-Host "`n==> $($f.dir)\$($f.name) ($($f.size))" -ForegroundColor Cyan
  # tải vào .part, chạy lại script sẽ tải tiếp phần còn thiếu
  & curl.exe -L --fail -C - -o "$out.part" $f.url
  if ($LASTEXITCODE -ne 0) { throw "Tải lỗi (chạy lại script để tải tiếp): $($f.url)" }
  Move-Item "$out.part" $out
}
Write-Host "`nXong. Khởi động lại ComfyUI để nó nhận model mới." -ForegroundColor Green
