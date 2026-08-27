#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
cd "$repository_root"

package_target_dir=${QUICKGUI_PACKAGE_TARGET_DIR:-"$repository_root/target/package-gate"}
if [[ $package_target_dir != /* ]]; then
  package_target_dir="$repository_root/$package_target_dir"
fi

allow_dirty_arg=""
if [[ ${QUICKGUI_PACKAGE_ALLOW_DIRTY:-0} == 1 ]]; then
  allow_dirty_arg=--allow-dirty
fi

cargo_command=(cargo)
if [[ -n ${QUICKGUI_PACKAGE_TOOLCHAIN:-} ]]; then
  cargo_command=(cargo "+$QUICKGUI_PACKAGE_TOOLCHAIN")
fi

winit_patch="patch.crates-io.quickgui-winit.path=\"$repository_root/vendor/winit\""
accesskit_patch="patch.crates-io.quickgui-accesskit-winit.path=\"$repository_root/vendor/accesskit_winit\""
cosmic_text_patch="patch.crates-io.quickgui-cosmic-text.path=\"$repository_root/vendor/cosmic_text\""
glyphon_patch="patch.crates-io.quickgui-glyphon.path=\"$repository_root/vendor/glyphon\""

CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/winit/Cargo.toml \
  ${allow_dirty_arg:+"$allow_dirty_arg"}
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/accesskit_winit/Cargo.toml \
  --no-verify \
  ${allow_dirty_arg:+"$allow_dirty_arg"} \
  --config "$winit_patch"
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/cosmic_text/Cargo.toml \
  ${allow_dirty_arg:+"$allow_dirty_arg"}
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/glyphon/Cargo.toml \
  ${allow_dirty_arg:+"$allow_dirty_arg"} \
  --config "$cosmic_text_patch"
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --locked \
  --no-verify \
  ${allow_dirty_arg:+"$allow_dirty_arg"} \
  --config "$winit_patch" \
  --config "$accesskit_patch" \
  --config "$cosmic_text_patch" \
  --config "$glyphon_patch"

package_dir="$package_target_dir/package"
winit_archive="$package_dir/quickgui-winit-0.30.13-quickgui.1.crate"
accesskit_archive="$package_dir/quickgui-accesskit-winit-0.33.2-quickgui.1.crate"
cosmic_text_archive="$package_dir/quickgui-cosmic-text-0.19.0-quickgui.1.crate"
glyphon_archive="$package_dir/quickgui-glyphon-0.12.0-quickgui.1.crate"
quickgui_archive="$package_dir/quickgui-0.1.0.crate"

for archive in "$winit_archive" "$accesskit_archive" "$cosmic_text_archive" "$glyphon_archive" "$quickgui_archive"; do
  if [[ ! -f $archive ]]; then
    echo "package-release-gate: missing archive $archive" >&2
    exit 1
  fi
done

scratch_dir=$(mktemp -d -t quickgui-package-gate.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT

tar -xzf "$winit_archive" -C "$scratch_dir"
tar -xzf "$accesskit_archive" -C "$scratch_dir"
tar -xzf "$cosmic_text_archive" -C "$scratch_dir"
tar -xzf "$glyphon_archive" -C "$scratch_dir"
tar -xzf "$quickgui_archive" -C "$scratch_dir"
cp -R tests/downstream_smoke "$scratch_dir/consumer"

required_files=(
  "quickgui-winit-0.30.13-quickgui.1/LICENSE"
  "quickgui-winit-0.30.13-quickgui.1/README.md"
  "quickgui-accesskit-winit-0.33.2-quickgui.1/LICENSE-APACHE"
  "quickgui-accesskit-winit-0.33.2-quickgui.1/README.md"
  "quickgui-cosmic-text-0.19.0-quickgui.1/LICENSE-APACHE"
  "quickgui-cosmic-text-0.19.0-quickgui.1/LICENSE-MIT"
  "quickgui-cosmic-text-0.19.0-quickgui.1/README.md"
  "quickgui-glyphon-0.12.0-quickgui.1/LICENSE-APACHE"
  "quickgui-glyphon-0.12.0-quickgui.1/LICENSE-MIT"
  "quickgui-glyphon-0.12.0-quickgui.1/LICENSE-ZLIB"
  "quickgui-glyphon-0.12.0-quickgui.1/README.md"
  "quickgui-0.1.0/LICENSE-MIT"
  "quickgui-0.1.0/LICENSE-APACHE"
  "quickgui-0.1.0/CHANGELOG.md"
  "quickgui-0.1.0/THIRD_PARTY_NOTICES.md"
  "quickgui-0.1.0/README.md"
  "quickgui-0.1.0/docs/releasing.md"
  "quickgui-0.1.0/tests/fixtures/fonts/Inter-LICENSE"
  "quickgui-0.1.0/tests/fixtures/fonts/NotoSans-LICENSE"
)
for relative_path in "${required_files[@]}"; do
  if [[ ! -f "$scratch_dir/$relative_path" ]]; then
    echo "package-release-gate: packaged file is missing: $relative_path" >&2
    exit 1
  fi
done
if [[ -d "$scratch_dir/quickgui-0.1.0/vendor" ]]; then
  echo "package-release-gate: the main archive must not duplicate vendored support sources" >&2
  exit 1
fi

packaged_winit_patch="patch.crates-io.quickgui-winit.path=\"$scratch_dir/quickgui-winit-0.30.13-quickgui.1\""
packaged_accesskit_patch="patch.crates-io.quickgui-accesskit-winit.path=\"$scratch_dir/quickgui-accesskit-winit-0.33.2-quickgui.1\""
packaged_cosmic_text_patch="patch.crates-io.quickgui-cosmic-text.path=\"$scratch_dir/quickgui-cosmic-text-0.19.0-quickgui.1\""
packaged_glyphon_patch="patch.crates-io.quickgui-glyphon.path=\"$scratch_dir/quickgui-glyphon-0.12.0-quickgui.1\""
packaged_quickgui_patch="patch.crates-io.quickgui.path=\"$scratch_dir/quickgui-0.1.0\""

CARGO_TARGET_DIR="$package_target_dir/downstream" "${cargo_command[@]}" check \
  --manifest-path "$scratch_dir/consumer/Cargo.toml" \
  --config "$packaged_winit_patch" \
  --config "$packaged_accesskit_patch" \
  --config "$packaged_cosmic_text_patch" \
  --config "$packaged_glyphon_patch" \
  --config "$packaged_quickgui_patch"

read -r winit_hash _ < <(shasum -a 256 "$winit_archive")
read -r accesskit_hash _ < <(shasum -a 256 "$accesskit_archive")
read -r cosmic_text_hash _ < <(shasum -a 256 "$cosmic_text_archive")
read -r glyphon_hash _ < <(shasum -a 256 "$glyphon_archive")
read -r quickgui_hash _ < <(shasum -a 256 "$quickgui_archive")

checksum_file="$package_dir/SHA256SUMS"
printf '%s  %s\n' \
  "$winit_hash" "$(basename "$winit_archive")" \
  "$accesskit_hash" "$(basename "$accesskit_archive")" \
  "$cosmic_text_hash" "$(basename "$cosmic_text_archive")" \
  "$glyphon_hash" "$(basename "$glyphon_archive")" \
  "$quickgui_hash" "$(basename "$quickgui_archive")" \
  > "$checksum_file"

printf '%s\n' \
  "QUICKGUI_PACKAGE_RESULT {\"winit_sha256\":\"${winit_hash}\",\"accesskit_sha256\":\"${accesskit_hash}\",\"cosmic_text_sha256\":\"${cosmic_text_hash}\",\"glyphon_sha256\":\"${glyphon_hash}\",\"quickgui_sha256\":\"${quickgui_hash}\",\"checksums\":\"SHA256SUMS\",\"required_files\":true,\"vendor_excluded\":true,\"downstream_check\":true,\"passed\":true}"
