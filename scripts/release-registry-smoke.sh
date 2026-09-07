#!/usr/bin/env bash

set -euo pipefail

if (( $# != 1 )); then
  echo "usage: release-registry-smoke.sh <version>" >&2
  exit 2
fi

release_version=$1
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)

scratch_dir=$(mktemp -d -t quickgui-registry-smoke.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT

rust_consumer="$scratch_dir/rust-consumer"
mkdir "$rust_consumer"
cargo +1.90.0 init --bin --name quickgui-release-smoke "$rust_consumer"
(
  cd "$rust_consumer"
  cargo +1.90.0 add "quickgui@=$release_version"
  CARGO_TARGET_DIR="$repository_root/target/release-registry-smoke" cargo +1.90.0 check
)

npm_consumer="$scratch_dir/npm-consumer"
mkdir "$npm_consumer"
(
  cd "$npm_consumer"
  bun init -y >/dev/null
  bun add \
    "@quickgui/native@$release_version" \
    "@quickgui/cli@$release_version"
  ./node_modules/.bin/quickgui init compiled-consumer --no-install
  (cd compiled-consumer && bun install && CGO_ENABLED=0 go mod tidy)
  ./node_modules/.bin/quickgui dev --project compiled-consumer --once --no-launch
  ./node_modules/.bin/quickgui --help | grep -F "QuickGUI CLI $release_version"
)

printf '%s\n' \
  "QUICKGUI_REGISTRY_SMOKE {\"version\":\"$release_version\",\"passed\":true}"
