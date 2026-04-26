#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-full}"

export PATH="/opt/homebrew/opt/rustup/bin:${HOME}/.cargo/bin:${PATH}"

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

assert_contains() {
  local file="$1"
  local text="$2"
  if ! grep -Fq "$text" "$file"; then
    printf 'Expected %s to contain: %s\n' "$file" "$text" >&2
    printf -- '--- %s ---\n' "$file" >&2
    sed -n '1,220p' "$file" >&2
    exit 1
  fi
}

case "$MODE" in
  smoke|full|live-provider)
    ;;
  *)
    printf 'usage: %s [smoke|full|live-provider]\n' "$0" >&2
    exit 2
    ;;
esac

cd "$ROOT_DIR"

run cargo fmt --check
run cargo test --workspace --locked

if [[ "$MODE" == "full" || "$MODE" == "live-provider" ]]; then
  run cargo clippy --workspace --all-targets -- -D warnings
fi

RUN_ROOT="${TMPDIR:-/tmp}/whalecode-test-runs/after-change-$(date +%Y%m%d-%H%M%S)-$$"
FIXTURE_REPO="$RUN_ROOT/fixture-repo"
WHALE_HOME="$RUN_ROOT/whale-home"
mkdir -p "$FIXTURE_REPO" "$WHALE_HOME"
printf '# Fixture\n' > "$FIXTURE_REPO/README.md"

STATUS_OUT="$RUN_ROOT/status.out"
BOOTSTRAP_OUT="$RUN_ROOT/bootstrap.out"
LOGS_OUT="$RUN_ROOT/logs.out"
NO_KEY_OUT="$RUN_ROOT/no-key.out"
NO_KEY_ERR="$RUN_ROOT/no-key.err"

run cargo run -p whalecode-cli --bin whale -- status > "$STATUS_OUT"
assert_contains "$STATUS_OUT" "command: whale"
assert_contains "$STATUS_OUT" "runtime: live_deepseek_tool_loop"
assert_contains "$STATUS_OUT" "session_store: jsonl"

BOOTSTRAP_SESSION="$RUN_ROOT/bootstrap-session.jsonl"
run cargo run -p whalecode-cli --bin whale -- run --bootstrap "inspect fixture" \
  --cwd "$FIXTURE_REPO" \
  --session "$BOOTSTRAP_SESSION" > "$BOOTSTRAP_OUT"
assert_contains "$BOOTSTRAP_OUT" "Bootstrap agent accepted the task"
assert_contains "$BOOTSTRAP_OUT" "session:"
test -s "$BOOTSTRAP_SESSION"

run cargo run -p whalecode-cli --bin whale -- logs --session "$BOOTSTRAP_SESSION" > "$LOGS_OUT"
assert_contains "$LOGS_OUT" "turn started index=1"
assert_contains "$LOGS_OUT" "tool output"
assert_contains "$LOGS_OUT" "assistant"

NO_KEY_SESSION="$RUN_ROOT/no-key-session.jsonl"
set +e
WHALE_SECRET_HOME="$WHALE_HOME" cargo run -p whalecode-cli --bin whale -- run "hi" \
  --cwd "$FIXTURE_REPO" \
  --session "$NO_KEY_SESSION" > "$NO_KEY_OUT" 2> "$NO_KEY_ERR"
NO_KEY_STATUS=$?
set -e
if [[ "$NO_KEY_STATUS" -eq 0 ]]; then
  printf 'Expected live run without a DeepSeek API key to fail.\n' >&2
  exit 1
fi
assert_contains "$NO_KEY_OUT" "workspace:"
assert_contains "$NO_KEY_OUT" "session:"
assert_contains "$NO_KEY_ERR" "DeepSeek API key is required"
test -s "$NO_KEY_SESSION"

if [[ "$MODE" == "live-provider" ]]; then
  if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
    printf 'live-provider mode requires DEEPSEEK_API_KEY.\n' >&2
    exit 2
  fi
  run cargo run -p whalecode-cli --bin whale -- model-smoke --model "${DEEPSEEK_MODEL:-deepseek-v4-flash}" "say hello"
fi

printf '\nAll %s checks passed. Runtime artifacts are in %s\n' "$MODE" "$RUN_ROOT"
