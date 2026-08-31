#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
cd "$repository_root"

package_version() {
  awk -F '"' '/^version = "/ { print $2; exit }' "$1"
}

npm_version() {
  node -e 'const fs = require("node:fs"); const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); process.stdout.write(manifest.version)' "$1"
}

rust_version=$(package_version Cargo.toml)
native_version=$(npm_version packages/native/package.json)
solid_version=$(npm_version packages/solid/package.json)
cli_version=$(npm_version packages/cli/package.json)

if [[ -z $rust_version ]]; then
  echo "release-metadata: could not read the QuickGUI crate version" >&2
  exit 1
fi
if [[ -z $native_version || $native_version != "$solid_version" || $native_version != "$cli_version" ]]; then
  echo "release-metadata: all @quickgui npm packages must have the same version" >&2
  echo "native=$native_version solid=$solid_version cli=$cli_version" >&2
  exit 1
fi

release_tag=${QUICKGUI_RELEASE_TAG:-}
if [[ -z $release_tag && ${GITHUB_REF_TYPE:-} == tag ]]; then
  release_tag=${GITHUB_REF_NAME:-}
fi
if [[ -n $release_tag ]]; then
  expected_tag="v$rust_version"
  if [[ $release_tag != "$expected_tag" ]]; then
    echo "release-metadata: tag $release_tag does not match crate version $rust_version" >&2
    exit 1
  fi
  if ! grep -Eq "^## ${rust_version//./\\.} - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md; then
    echo "release-metadata: CHANGELOG.md has no dated $rust_version release section" >&2
    exit 1
  fi
fi

output_file=${1:-${GITHUB_OUTPUT:-}}
if [[ -n $output_file ]]; then
  {
    printf 'version=%s\n' "$rust_version"
    printf 'rust-version=%s\n' "$rust_version"
    printf 'npm-version=%s\n' "$native_version"
  } >> "$output_file"
fi

printf '%s\n' \
  "QUICKGUI_RELEASE_METADATA {\"rust_version\":\"$rust_version\",\"npm_version\":\"$native_version\",\"tag\":\"$release_tag\",\"passed\":true}"
