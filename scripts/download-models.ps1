# Tải model vẽ hình vào thư mục models của ComfyUI.
# Tổng ~12.5GB. Tải lại được nếu đứt mạng (curl -C -).
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\download-models.ps1 -ComfyModels "D:\ComfyUI_windows_portable\ComfyUI\models"
#
# Model: FLUX.2 [klein] 4B distilled FP8 (Apache 2.0) + text encoder Qwen3 4B + FLUX.2 VAE

param(
  [Parameter(Mandatory = $true)][string]$ComfyModels
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
