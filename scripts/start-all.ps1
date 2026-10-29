# Mở mọi dịch vụ app cần rồi chạy app: Ollama, ComfyUI, VieNeu-TTS, AI Video Studio.
# Dịch vụ nào đang chạy rồi thì bỏ qua; dịch vụ mới mở trong cửa sổ riêng để xem log và tắt bằng Ctrl+C.
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\start-all.ps1
#   powershell -ExecutionPolicy Bypass -File .\scripts\start-all.ps1 -ComfyDir "D:\ComfyUI_windows_portable" -NoApp

param(
  [string]$ComfyDir = (Join-Path $env:USERPROFILE "Downloads\ComfyUI_windows_portable"),
  [int]$TtsPort = 8001,
  # Chỉ mở dịch vụ, không chạy app (ví dụ khi mở app bằng file .exe đã build).
  [switch]$NoApp
)

$repo = Split-Path -Parent $PSScriptRoot

function Test-Url([string]$Url) {
  try { Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2 | Out-Null; return $true } catch { return $false }
}

function Wait-Url([string]$Name, [string]$Url, [int]$Seconds) {
  Write-Host -NoNewline "  Chờ $Name"
  for ($i = 0; $i -lt $Seconds; $i += 2) {
    if (Test-Url $Url) { Write-Host " sẵn sàng" -ForegroundColor Green; return $true }
    Write-Host -NoNewline "."
    Start-Sleep -Seconds 2
  }
  Write-Host " chưa lên sau $Seconds giây, xem cửa sổ của $Name" -ForegroundColor Yellow
  return $false
}

$services = @(
  @{ Name = "Ollama"; Url = "http://127.0.0.1:11434/api/tags"; Wait = 30; Start = {
      if (-not (Get-Command ollama -ErrorAction SilentlyContinue)) { throw "Chưa cài Ollama (https://ollama.com)." }
      Start-Process -FilePath "ollama" -ArgumentList "serve" -WindowStyle Minimized
    } },
  @{ Name = "ComfyUI"; Url = "http://127.0.0.1:8188/system_stats"; Wait = 120; Start = {
      $bat = Join-Path $ComfyDir "run_nvidia_gpu.bat"
      if (-not (Test-Path $bat)) { throw "Không thấy $bat. Chạy lại với -ComfyDir <thư mục ComfyUI portable>." }
      Start-Process -FilePath $bat -WorkingDirectory $ComfyDir
    } },
  @{ Name = "VieNeu-TTS"; Url = "http://127.0.0.1:$TtsPort/health"; Wait = 240; Start = {
      $script = Join-Path $PSScriptRoot "start-tts.ps1"
      Start-Process -FilePath "powershell" -ArgumentList @("-NoExit", "-ExecutionPolicy", "Bypass", "-File", "`"$script`"", "-Port", $TtsPort)
    } }
)

Write-Host "Mở dịch vụ" -ForegroundColor Cyan
$started = @()
foreach ($s in $services) {
  if (Test-Url $s.Url) { Write-Host "  $($s.Name): đang chạy" -ForegroundColor Green; continue }
  try {
    & $s.Start
    Write-Host "  $($s.Name): đang mở"
    $started += $s
  } catch {
    Write-Host "  $($s.Name): $($_.Exception.Message)" -ForegroundColor Red
  }
}
# Chờ sau khi đã mở hết, để các dịch vụ khởi động song song. TTS lần đầu tải model nên lâu nhất.
foreach ($s in $started) { Wait-Url $s.Name $s.Url $s.Wait | Out-Null }

if ($NoApp) { return }
Write-Host "`nChạy AI Video Studio (Ctrl+C để tắt app; các dịch vụ vẫn chạy trong cửa sổ riêng)" -ForegroundColor Cyan
Set-Location $repo
if (Get-Command yarn -ErrorAction SilentlyContinue) { yarn run app } else { npm run app }
