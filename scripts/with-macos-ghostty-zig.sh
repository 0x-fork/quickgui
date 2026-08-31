#!/usr/bin/env bash

set -euo pipefail

if (( $# == 0 )); then
  echo "usage: with-macos-ghostty-zig.sh <command> [arguments...]" >&2
  exit 2
fi

if [[ $(uname -s) != Darwin ]]; then
  exec "$@"
fi

selected_developer=$(/usr/bin/xcode-select -p)
stable_developer=/Applications/Xcode.app/Contents/Developer
stable_clang=$stable_developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/clang
if [[ ${QUICKGUI_ALLOW_BETA_XCODE:-0} != 1 ]]; then
  case $selected_developer in
    *[Bb][Ee][Tt][Aa]*)
      if [[ -x $stable_clang ]]; then
        export DEVELOPER_DIR=$stable_developer
        export CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=$stable_clang
        export CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=$stable_clang
        echo "with-macos-ghostty-zig: using stable Xcode at $stable_developer"
      fi
      ;;
  esac
fi

zig_candidates=()
if [[ -n ${QUICKGUI_ZIG:-} ]]; then
  zig_candidates+=("$QUICKGUI_ZIG")
fi
current_zig=$(command -v zig || true)
if [[ -n $current_zig ]]; then
  zig_candidates+=("$current_zig")
fi
if command -v mise >/dev/null 2>&1; then
  mise_root=$(mise where zig@0.15.2 2>/dev/null || true)
  if [[ -n $mise_root ]]; then
    zig_candidates+=("$mise_root/zig" "$mise_root/bin/zig")
  fi
fi

zig_command=""
for candidate in "${zig_candidates[@]}"; do
  if [[ -x $candidate && $("$candidate" version) == 0.15.2 ]]; then
    zig_command=$candidate
    break
  fi
done
if [[ -z $zig_command ]]; then
  echo "with-macos-ghostty-zig: Zig 0.15.2 is required" >&2
  exit 1
fi

export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-13.0}
sdk=$(/usr/bin/xcrun --sdk macosx --show-sdk-path)
export SDKROOT=$sdk
zig_root=$(CDPATH= cd -- "$(dirname -- "$zig_command")" && pwd)
bundled_lib_system=""
for candidate in \
  "$zig_root/lib/libc/darwin/libSystem.tbd" \
  "$zig_root/../lib/libc/darwin/libSystem.tbd"
do
  if [[ -f $candidate ]]; then
    bundled_lib_system=$candidate
    break
  fi
done
if [[ -z $bundled_lib_system ]]; then
  echo "with-macos-ghostty-zig: could not locate Zig's bundled libSystem.tbd" >&2
  exit 1
fi

# Zig 0.15.2 cannot parse the newer libSystem.tbd shipped by current Xcode SDKs. Preserve the
# real headers and frameworks while exposing Zig's compatible libSystem only to Zig subprocesses.
scratch_dir=$(mktemp -d -t quickgui-ghostty.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT
overlay="$scratch_dir/MacOSX.sdk"
mkdir -p "$overlay/usr/lib" "$overlay/System/Library"
ln -s "$sdk/usr/include" "$overlay/usr/include"
ln -s "$bundled_lib_system" "$overlay/usr/lib/libSystem.tbd"
ln -s "$sdk/System/Library/Frameworks" "$overlay/System/Library/Frameworks"
for settings in SDKSettings.json SDKSettings.plist; do
  if [[ -e $sdk/$settings ]]; then
    ln -s "$sdk/$settings" "$overlay/$settings"
  fi
done

xcrun_directory="$scratch_dir/xcrun-bin"
zig_directory="$scratch_dir/zig-bin"
mkdir "$xcrun_directory" "$zig_directory"
cat > "$xcrun_directory/xcrun" <<'EOF'
#!/bin/sh
if [ "$1" = "--sdk" ] && [ "$3" = "--show-sdk-path" ]; then
  printf '%s\n' "$QUICKGUI_MACOS_SDK_OVERLAY"
  exit 0
fi
exec /usr/bin/xcrun "$@"
EOF
cat > "$zig_directory/zig" <<'EOF'
#!/bin/sh
PATH="$QUICKGUI_XCRUN_DIRECTORY:$PATH" exec "$QUICKGUI_REAL_ZIG" "$@"
EOF
chmod 0755 "$xcrun_directory/xcrun" "$zig_directory/zig"

export QUICKGUI_MACOS_SDK_OVERLAY=$overlay
export QUICKGUI_REAL_ZIG=$zig_command
export QUICKGUI_XCRUN_DIRECTORY=$xcrun_directory
export PATH="$zig_directory:$PATH"
script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export ZIG_GLOBAL_CACHE_DIR=${ZIG_GLOBAL_CACHE_DIR:-"$script_directory/../target/zig-global-cache-0.15.2"}

echo "with-macos-ghostty-zig: using Zig 0.15.2 at $zig_command"
"$@"
