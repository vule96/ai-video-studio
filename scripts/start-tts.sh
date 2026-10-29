#!/usr/bin/env bash
# Runs VieNeu-TTS (Vietnamese voices, Apache 2.0) for the app's render step.
# First run creates a private Python env and installs vieneu (~1 GB); the model downloads on start.
# Runs on CPU so the GPU stays free for ComfyUI. Keep it open while rendering, Ctrl+C to stop.
#   ./scripts/start-tts.sh [port]
set -euo pipefail

port="${1:-8001}"
venv="$HOME/.aivideostudio/vieneu-venv"
[ -x "$venv/bin/python" ] || python3 -m venv "$venv"
# Check the import, not just the folder: an earlier install may have been interrupted.
"$venv/bin/python" -c "import vieneu" 2>/dev/null || {
  "$venv/bin/python" -m pip install --upgrade pip
  "$venv/bin/python" -m pip install "vieneu==3.8.3"
}

echo "VieNeu-TTS: http://127.0.0.1:$port, wait for 'ready' before rendering."
HOST=127.0.0.1 PORT="$port" VIENEU_BACKEND=onnx VIENEU_DEVICE=cpu exec "$venv/bin/python" -m apps.openai_speech
