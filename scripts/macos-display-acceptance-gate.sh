#!/usr/bin/env bash

set -euo pipefail

if [[ $(uname -s) != Darwin ]]; then
  echo "macos-display-acceptance-gate: this live AppKit/WindowServer probe requires macOS" >&2
  exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
cd "$repository_root"

target_dir=${CARGO_TARGET_DIR:-"$repository_root/target"}
if [[ $target_dir != /* ]]; then
  target_dir="$repository_root/$target_dir"
fi
binary="$target_dir/release/examples/displays"

require_secondary=${QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_SECONDARY:-1}
require_mixed_scale=${QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_MIXED_SCALE:-0}
probe_log=$(mktemp -t quickgui-display-acceptance.XXXXXX)
trap 'rm -f "$probe_log"' EXIT

cargo build --release --example displays --locked

set +e
QUICKGUI_DISPLAY_ACCEPTANCE=1 \
QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_SECONDARY="$require_secondary" \
QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_MIXED_SCALE="$require_mixed_scale" \
  "$binary" 2>&1 | tee "$probe_log"
probe_status=${PIPESTATUS[0]}
set -e

if [[ $probe_status -ne 0 ]]; then
  echo "macos-display-acceptance-gate: native display probe exited with status $probe_status" >&2
  exit "$probe_status"
fi
if ! grep -q '^QUICKGUI_DISPLAY_ACCEPTANCE_RESULT .*"passed":true' "$probe_log"; then
  echo "macos-display-acceptance-gate: no passing native display result was emitted" >&2
  exit 1
fi
