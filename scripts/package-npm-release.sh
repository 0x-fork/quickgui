#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
cd "$repository_root"
"$script_dir/release-metadata.sh" >/dev/null

output_dir=${1:-target/npm-release}
if [[ $output_dir != /* ]]; then
  output_dir="$repository_root/$output_dir"
fi
mkdir -p "$output_dir"
find "$output_dir" -maxdepth 1 -type f \( -name 'quickgui-*.tgz' -o -name 'NPM_SHA256SUMS' \) -delete

manifest_version() {
  node -e 'const fs = require("node:fs"); const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); process.stdout.write(manifest.version)' "$1"
}

release_version=$(manifest_version package.json)
native_version=$(manifest_version packages/native/package.json)
ui_version=$(manifest_version packages/ui/package.json)
cli_version=$(manifest_version packages/cli/package.json)
if [[ $release_version != "$native_version" ]] || \
   [[ $release_version != "$ui_version" ]] || \
   [[ $release_version != "$cli_version" ]]
then
  echo "package-npm-release: all @quickgui packages must match root version $release_version" >&2
  exit 1
fi

arm64_binary=packages/native/lib/darwin-arm64/libquickgui_host.a
x64_binary=packages/native/lib/darwin-x64/libquickgui_host.a
for binary in "$arm64_binary" "$x64_binary"; do
  if [[ ! -f $binary ]]; then
    echo "package-npm-release: missing native binary $binary" >&2
    exit 1
  fi
done
if [[ $(uname -s) == Darwin ]]; then
  if ! lipo -verify_arch arm64 "$arm64_binary"; then
    echo "package-npm-release: $arm64_binary is not a macOS arm64 binary" >&2
    exit 1
  fi
  if ! lipo -verify_arch x86_64 "$x64_binary"; then
    echo "package-npm-release: $x64_binary is not a macOS x64 binary" >&2
    exit 1
  fi
fi

for package_name in native ui cli; do
  (cd "packages/$package_name" && bun pm pack --destination "$output_dir" --quiet)
done

native_archive="$output_dir/quickgui-native-${release_version}.tgz"
ui_archive="$output_dir/quickgui-ui-${release_version}.tgz"
cli_archive="$output_dir/quickgui-cli-${release_version}.tgz"
for archive in "$native_archive" "$ui_archive" "$cli_archive"; do
  if [[ ! -f $archive ]]; then
    echo "package-npm-release: missing archive $archive" >&2
    exit 1
  fi
done

verify_manifest() {
  local archive=$1
  local expected_name=$2
  local manifest_file=$3
  tar -xOf "$archive" package/package.json > "$manifest_file"
  if [[ $(jq -r '.name' "$manifest_file") != "$expected_name" || \
        $(jq -r '.version' "$manifest_file") != "$release_version" ]]
  then
    echo "package-npm-release: unexpected name or version in $archive" >&2
    exit 1
  fi
  if ! jq -e '.os == ["darwin"] and .publishConfig.access == "public"' "$manifest_file" >/dev/null; then
    echo "package-npm-release: $archive must be a public macOS package" >&2
    exit 1
  fi
}

manifest_dir=$(mktemp -d -t quickgui-npm-manifests.XXXXXX)
trap 'rm -rf "$manifest_dir"' EXIT
verify_manifest "$native_archive" '@quickgui/native' "$manifest_dir/native.json"
verify_manifest "$ui_archive" '@quickgui/ui' "$manifest_dir/ui.json"
verify_manifest "$cli_archive" '@quickgui/cli' "$manifest_dir/cli.json"

if ! jq -e --arg version "$release_version" \
  '.dependencies["@quickgui/native"] == $version' \
  "$manifest_dir/ui.json" >/dev/null
then
  echo "package-npm-release: UI must depend on the exact native release" >&2
  exit 1
fi
if ! jq -e --arg version "$release_version" \
  '.dependencies["@quickgui/native"] == $version and
   .dependencies["@quickgui/ui"] == $version and
   .bin.quickgui == "src/cli.ts"' \
  "$manifest_dir/cli.json" >/dev/null
then
  echo "package-npm-release: CLI release dependencies or binary are incorrect" >&2
  exit 1
fi

native_entries=$(tar -tzf "$native_archive")
for entry in \
  package/lib/darwin-arm64/libquickgui_host.a \
  package/lib/darwin-arm64/link.json \
  package/lib/darwin-x64/libquickgui_host.a \
  package/lib/darwin-x64/link.json
do
  if ! grep -Fxq "$entry" <<< "$native_entries"; then
    echo "package-npm-release: native archive is missing $entry" >&2
    exit 1
  fi
done
native_binary_count=$(grep -Ec 'libquickgui_host\.a$' <<< "$native_entries" || true)
if [[ $native_binary_count != 2 ]]; then
  echo "package-npm-release: native archive must contain exactly two host archives" >&2
  exit 1
fi

checksum_file="$output_dir/NPM_SHA256SUMS"
(
  cd "$output_dir"
  shasum -a 256 \
    "$(basename "$native_archive")" \
    "$(basename "$ui_archive")" \
    "$(basename "$cli_archive")"
) > "$checksum_file"

printf '%s\n' \
  "QUICKGUI_NPM_PACKAGE_RESULT {\"version\":\"$release_version\",\"native\":\"$(basename "$native_archive")\",\"ui\":\"$(basename "$ui_archive")\",\"cli\":\"$(basename "$cli_archive")\",\"checksums\":\"$(basename "$checksum_file")\",\"passed\":true}"
