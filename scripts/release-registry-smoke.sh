#!/usr/bin/env bash

set -euo pipefail

if (( $# != 2 )); then
  echo "usage: release-registry-smoke.sh <rust-version> <npm-version>" >&2
  exit 2
fi

rust_release_version=$1
npm_release_version=$2
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)

scratch_dir=$(mktemp -d -t quickgui-registry-smoke.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT

rust_consumer="$scratch_dir/rust-consumer"
mkdir "$rust_consumer"
cargo +1.90.0 init --bin --name quickgui-release-smoke "$rust_consumer"
(
  cd "$rust_consumer"
  cargo +1.90.0 add "quickgui@=$rust_release_version"
  CARGO_TARGET_DIR="$repository_root/target/release-registry-smoke" cargo +1.90.0 check
)

npm_consumer="$scratch_dir/npm-consumer"
mkdir "$npm_consumer"
(
  cd "$npm_consumer"
  bun init -y >/dev/null
  bun add \
    "@quickgui/native@$npm_release_version" \
    "@quickgui/solid@$npm_release_version" \
    "@quickgui/cli@$npm_release_version"
  bun -e "await import('@quickgui/native'); await import('@quickgui/solid'); console.log('QuickGUI package imports passed')"
  ./node_modules/.bin/quickgui --help | grep -F "QuickGUI CLI $npm_release_version"
)

printf '%s\n' \
  "QUICKGUI_REGISTRY_SMOKE {\"rust_version\":\"$rust_release_version\",\"npm_version\":\"$npm_release_version\",\"passed\":true}"
