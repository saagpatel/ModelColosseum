#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="model-colosseum"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORIGINAL_HOME="$HOME"

if [[ -n "${MODEL_COLOSSEUM_HOME:-}" ]]; then
  export HOME="$MODEL_COLOSSEUM_HOME"
  export CARGO_HOME="${CARGO_HOME:-$ORIGINAL_HOME/.cargo}"
  export RUSTUP_HOME="${RUSTUP_HOME:-$ORIGINAL_HOME/.rustup}"
  mkdir -p "$HOME"
fi

cd "$ROOT_DIR"
pkill -x "$APP_NAME" >/dev/null 2>&1 || true

run_dev() {
  pnpm tauri dev
}

case "$MODE" in
  run)
    run_dev
    ;;
  --debug|debug)
    pnpm tauri build --debug --no-bundle
    lldb -- "$ROOT_DIR/src-tauri/target/debug/$APP_NAME"
    ;;
  --logs|logs)
    run_dev &
    /usr/bin/log stream --info --style compact --predicate "process == \"$APP_NAME\""
    ;;
  --telemetry|telemetry)
    run_dev &
    /usr/bin/log stream --info --style compact --predicate 'process == "model-colosseum"'
    ;;
  --verify|verify)
    run_dev &
    DEV_PID=$!
    for _ in {1..90}; do
      if pgrep -x "$APP_NAME" >/dev/null; then
        exit 0
      fi
      sleep 1
    done
    kill "$DEV_PID" >/dev/null 2>&1 || true
    echo "ModelColosseum did not launch within 90 seconds" >&2
    exit 1
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
