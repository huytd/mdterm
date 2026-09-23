#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
REC_BIN="$REPO_ROOT/target/release/term-rec"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
SCENARIOS_DIR="$SCRIPT_DIR/scenarios"

echo "=== Building term-rec ==="
cargo build --release -p term-rec --manifest-path "$REPO_ROOT/Cargo.toml"

mkdir -p "$FIXTURES_DIR"

echo "=== Recording non-tmux scenarios ==="
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/box-drawing" -- /bin/bash "$SCENARIOS_DIR/box-drawing.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/powerline" -- /bin/bash "$SCENARIOS_DIR/powerline.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/wide-chars" -- /bin/bash "$SCENARIOS_DIR/wide-chars.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/colors" -- /bin/bash "$SCENARIOS_DIR/colors.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/scroll-region" -- /bin/bash "$SCENARIOS_DIR/scroll-region.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/altscreen" -- /bin/bash "$SCENARIOS_DIR/altscreen.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/sync-output" -- /bin/bash "$SCENARIOS_DIR/sync-output.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/curses-app" -- python3 "$SCENARIOS_DIR/curses-app.py"

echo "=== Recording tmux scenarios ==="
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/tmux-splits" --tmux -- /bin/bash "$SCENARIOS_DIR/tmux-splits.sh"
"$REC_BIN" --cols 80 --rows 24 --out "$FIXTURES_DIR/tmux-resize" --tmux --resize-after-ms 400 --resize 100x30 -- /bin/bash "$SCENARIOS_DIR/tmux-resize.sh"

echo "=== All fixtures recorded successfully in $FIXTURES_DIR ==="
