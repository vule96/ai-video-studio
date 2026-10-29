# Chạy VieNeu-TTS (giọng đọc tiếng Việt, Apache 2.0) cho bước Dựng phim của app.
# Lần đầu: tạo môi trường Python riêng và cài vieneu (~1GB), lúc khởi động tải model từ Hugging Face.
# Chạy bằng CPU để GPU dành cho ComfyUI. Giữ cửa sổ này mở trong lúc dựng phim, Ctrl+C để tắt.
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\start-tts.ps1
#
# Cần Python 3.10 trở lên trong PATH.

param([int]$Port = 8001)
# No $ErrorActionPreference = "Stop": Windows PowerShell 5.1 then treats any stderr output of
# python/pip (warnings, logs) as a fatal error. Exit codes are checked explicitly instead.

$venv = Join-Path $env:LOCALAPPDATA "AIVideoStudio\vieneu-venv"
$py = Join-Path $venv "Scripts\python.exe"
if (-not (Test-Path $py)) {
  Write-Host "Tạo môi trường Python tại $venv" -ForegroundColor Cyan
  python -m venv $venv
  if ($LASTEXITCODE -ne 0 -or -not (Test-Path $py)) { throw "Không tạo được venv. Kiểm tra: python --version (cần 3.10 trở lên)." }
}
# Kiểm tra gói đã cài thay vì chỉ có venv: lần cài trước có thể đã đứt giữa chừng.
& $py -c "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('vieneu') else 1)"
if ($LASTEXITCODE -ne 0) {
  Write-Host "Cài vieneu (lần đầu mất vài phút)" -ForegroundColor Cyan
  & $py -m pip install --upgrade pip
  & $py -m pip install "vieneu==3.8.3"
  if ($LASTEXITCODE -ne 0) { throw "Cài vieneu thất bại, xem lỗi pip ở trên." }
}

# Chỉ nghe trên máy này (mặc định của server là mọi địa chỉ mạng).
$env:HOST = "127.0.0.1"
$env:PORT = "$Port"
$env:VIENEU_BACKEND = "onnx"
$env:VIENEU_DEVICE = "cpu"
Write-Host "VieNeu-TTS: http://127.0.0.1:$Port , chờ dòng 'ready' rồi dùng app." -ForegroundColor Green
& $py -m apps.openai_speech
