#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="model-colosseum"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BINARY="$ROOT_DIR/src-tauri/target/debug/$APP_NAME"
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

find_repo_app_pid() {
  local pid executable
  while read -r pid; do
    [[ -n "$pid" ]] || continue
    executable="$(
      { lsof -a -p "$pid" -d txt -Fn 2>/dev/null || true; } \
        | sed -n 's/^n//p' \
        | head -n 1
    )"
    if [[ "$executable" == "$APP_BINARY" ]]; then
      echo "$pid"
      return 0
    fi
  done < <(pgrep -x "$APP_NAME" || true)
  return 1
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
    DEV_LOG="${TMPDIR:-/tmp}/model-colosseum-dev.log"
    nohup pnpm tauri dev >"$DEV_LOG" 2>&1 &
    DEV_PID=$!
    for _ in {1..90}; do
      if APP_PID="$(find_repo_app_pid)"; then
        echo "ModelColosseum launched from $APP_BINARY (pid $APP_PID)"
        echo "Dev log: $DEV_LOG"
        exit 0
      fi
      sleep 1
    done
    kill "$DEV_PID" >/dev/null 2>&1 || true
    echo "ModelColosseum did not launch from $APP_BINARY within 90 seconds" >&2
    echo "Dev log: $DEV_LOG" >&2
    exit 1
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
