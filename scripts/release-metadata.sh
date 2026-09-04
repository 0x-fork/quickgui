#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
cd "$repository_root"

cargo_version() {
  awk -F '"' '/^version = "/ { print $2; exit }' "$1"
}

manifest_version() {
  node -e 'const fs = require("node:fs"); const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); process.stdout.write(manifest.version)' "$1"
}

template_dependency() {
  awk -F '"' -v package_name="$2" '$2 == package_name { print $4; exit }' "$1"
}

assert_version() {
  local label=$1
  local actual=$2
  local expected=$3
  if [[ $actual != "$expected" ]]; then
    echo "release-metadata: $label is $actual; expected $expected" >&2
    exit 1
  fi
}

release_version=$(manifest_version package.json)
if [[ -z $release_version ]]; then
  echo "release-metadata: could not read the root package.json version" >&2
  exit 1
fi

assert_version "quickgui crate version" "$(cargo_version Cargo.toml)" "$release_version"
assert_version "quickgui-system crate version" "$(cargo_version crates/quickgui-system/Cargo.toml)" "$release_version"
assert_version "native binding crate version" "$(cargo_version packages/native/Cargo.toml)" "$release_version"
assert_version "@quickgui/native version" "$(manifest_version packages/native/package.json)" "$release_version"
assert_version "@quickgui/solid version" "$(manifest_version packages/solid/package.json)" "$release_version"
assert_version "@quickgui/cli version" "$(manifest_version packages/cli/package.json)" "$release_version"
assert_version "CLI_VERSION" "$(sed -n 's/^export const CLI_VERSION = "\([^"]*\)";$/\1/p' packages/cli/src/cli.ts)" "$release_version"
assert_version "Solid template native dependency" "$(template_dependency packages/cli/templates/solid/package.json @quickgui/native)" "^$release_version"
assert_version "Solid template renderer dependency" "$(template_dependency packages/cli/templates/solid/package.json @quickgui/solid)" "^$release_version"
assert_version "Solid template CLI dependency" "$(template_dependency packages/cli/templates/solid/package.json @quickgui/cli)" "^$release_version"

release_tag=${QUICKGUI_RELEASE_TAG:-}
if [[ -z $release_tag && ${GITHUB_REF_TYPE:-} == tag ]]; then
  release_tag=${GITHUB_REF_NAME:-}
fi
if [[ -n $release_tag ]]; then
  expected_tag="v$release_version"
  if [[ $release_tag != "$expected_tag" ]]; then
    echo "release-metadata: tag $release_tag does not match root package version $release_version" >&2
    exit 1
  fi
  if ! grep -Eq "^## ${release_version//./\\.} - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md; then
    echo "release-metadata: CHANGELOG.md has no dated $release_version release section" >&2
    exit 1
  fi
fi

output_file=${1:-${GITHUB_OUTPUT:-}}
if [[ -n $output_file ]]; then
  printf 'version=%s\n' "$release_version" >> "$output_file"
fi

printf '%s\n' \
  "QUICKGUI_RELEASE_METADATA {\"version\":\"$release_version\",\"tag\":\"$release_tag\",\"passed\":true}"
