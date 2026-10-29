#!/usr/bin/env bash
# Starts everything the app needs, then the app: Ollama, ComfyUI, VieNeu-TTS, AI Video Studio (macOS / Linux / Git Bash).
# Services already running are skipped; new ones run in the background with logs in ~/.aivideostudio/logs.
#   ./scripts/start-all.sh                       # ComfyUI Desktop on macOS, or COMFY_DIR=<ComfyUI folder>
#   COMFY_DIR=~/ComfyUI ./scripts/start-all.sh --no-app
set -uo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
logs="$HOME/.aivideostudio/logs"
mkdir -p "$logs"
tts_port="${TTS_PORT:-8001}"
comfy_port="${COMFY_PORT:-8188}"

up() { curl -fs --max-time 2 "$1" >/dev/null; }

wait_for() { # <name> <url> <seconds>
  printf '  Waiting for %s' "$1"
  for ((i = 0; i < $3; i += 2)); do
    if up "$2"; then echo " ready"; return 0; fi
    printf '.'; sleep 2
  done
  echo " not up after $3 s, see $logs"
}

echo "Starting services"
pending=()

if up http://127.0.0.1:11434/api/tags; then echo "  Ollama: running"
elif command -v ollama >/dev/null; then
  nohup ollama serve >"$logs/ollama.log" 2>&1 &
  echo "  Ollama: starting"; pending+=("Ollama|http://127.0.0.1:11434/api/tags|30")
else echo "  Ollama: not installed (https://ollama.com)"; fi

if up "http://127.0.0.1:$comfy_port/system_stats" || up http://127.0.0.1:8000/system_stats; then echo "  ComfyUI: running"
elif [ -n "${COMFY_DIR:-}" ]; then
  (cd "$COMFY_DIR" && nohup python3 main.py --port "$comfy_port" >"$logs/comfyui.log" 2>&1 &)
  echo "  ComfyUI: starting from $COMFY_DIR"; pending+=("ComfyUI|http://127.0.0.1:$comfy_port/system_stats|120")
elif [ "$(uname)" = Darwin ] && open -Ra ComfyUI 2>/dev/null; then
  open -a ComfyUI
  echo "  ComfyUI: opening ComfyUI Desktop"; pending+=("ComfyUI|http://127.0.0.1:8000/system_stats|120")
else echo "  ComfyUI: set COMFY_DIR=<ComfyUI folder> or install ComfyUI Desktop"; fi

if up "http://127.0.0.1:$tts_port/health"; then echo "  VieNeu-TTS: running"
else
  nohup "$repo/scripts/start-tts.sh" "$tts_port" >"$logs/vieneu.log" 2>&1 &
  echo "  VieNeu-TTS: starting"; pending+=("VieNeu-TTS|http://127.0.0.1:$tts_port/health|240")
fi

# Wait only after everything is launched, so the services boot in parallel.
for p in ${pending[@]+"${pending[@]}"}; do IFS='|' read -r name url secs <<<"$p"; wait_for "$name" "$url" "$secs"; done

[ "${1:-}" = "--no-app" ] && exit 0
echo
echo "Starting AI Video Studio (Ctrl+C stops the app; services keep running)"
cd "$repo" || exit 1
if command -v yarn >/dev/null; then exec yarn run app; else exec npm run app; fi
