#!/usr/bin/env bash

set -euo pipefail

case ${1:-} in
  --check)
    publish=0
    ;;
  --publish)
    publish=1
    ;;
  *)
    echo "usage: release-npm.sh --check|--publish [archive-directory]" >&2
    exit 2
    ;;
esac

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
cd "$repository_root"
"$script_dir/release-metadata.sh" >/dev/null

archive_dir=${2:-target/npm-release}
if [[ $archive_dir != /* ]]; then
  archive_dir="$repository_root/$archive_dir"
fi

manifest_version() {
  node -e 'const fs = require("node:fs"); const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); process.stdout.write(manifest.version)' "$1"
}

npm_release_version=$(manifest_version package.json)
scratch_dir=$(mktemp -d -t quickgui-npm-release.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT

package_is_published() {
  local package_name=$1
  local registry_name=$2
  local archive=$3
  local response_file="$scratch_dir/${registry_name}.json"
  local http_status
  local registry_integrity
  local local_integrity

  if ! http_status=$(curl \
    --silent \
    --show-error \
    --location \
    --retry 5 \
    --retry-all-errors \
    --output "$response_file" \
    --write-out '%{http_code}' \
    "https://registry.npmjs.org/@quickgui%2F$registry_name")
  then
    echo "release-npm: registry request failed for $package_name" >&2
    return 2
  fi

  case $http_status in
    200)
      registry_integrity=$(jq -r --arg version "$npm_release_version" \
        '.versions[$version].dist.integrity // empty' "$response_file")
      if [[ -z $registry_integrity ]]; then
        return 1
      fi
      local_integrity="sha512-$(openssl dgst -sha512 -binary "$archive" | openssl base64 -A)"
      if [[ $registry_integrity != "$local_integrity" ]]; then
        echo "release-npm: $package_name $npm_release_version is public with different bytes" >&2
        return 2
      fi
      return 0
      ;;
    404)
      return 1
      ;;
    *)
      echo "release-npm: registry returned HTTP $http_status for $package_name" >&2
      return 2
      ;;
  esac
}

wait_for_package() {
  local package_name=$1
  local registry_name=$2
  local archive=$3
  local status

  for _attempt in {1..60}; do
    if package_is_published "$package_name" "$registry_name" "$archive"; then
      return 0
    else
      status=$?
      if (( status != 1 )); then
        return "$status"
      fi
    fi
    sleep 5
  done

  echo "release-npm: timed out waiting for $package_name $npm_release_version" >&2
  return 1
}

if (( publish == 1 )); then
  node_version=$(node --version | sed 's/^v//')
  npm_cli_version=$(npm --version)
  IFS=. read -r node_major node_minor _ <<< "$node_version"
  IFS=. read -r npm_major npm_minor _ <<< "$npm_cli_version"
  if (( node_major < 22 || (node_major == 22 && node_minor < 14) )); then
    echo "release-npm: npm trusted publishing requires Node 22.14 or newer" >&2
    exit 1
  fi
  if (( npm_major < 11 || (npm_major == 11 && npm_minor < 5) )); then
    echo "release-npm: npm trusted publishing requires npm 11.5.1 or newer" >&2
    exit 1
  fi
  if [[ -z ${ACTIONS_ID_TOKEN_REQUEST_URL:-} && -z ${NODE_AUTH_TOKEN:-} ]]; then
    echo "release-npm: GitHub OIDC or NODE_AUTH_TOKEN authentication is required" >&2
    exit 1
  fi
fi

package_names=(
  '@quickgui/native'
  '@quickgui/cli'
)
registry_names=(native cli)
archives=(
  "$archive_dir/quickgui-native-${npm_release_version}.tgz"
  "$archive_dir/quickgui-cli-${npm_release_version}.tgz"
)

for index in "${!package_names[@]}"; do
  package_name=${package_names[$index]}
  registry_name=${registry_names[$index]}
  archive=${archives[$index]}
  if [[ ! -f $archive ]]; then
    echo "release-npm: missing archive $archive" >&2
    exit 1
  fi

  if package_is_published "$package_name" "$registry_name" "$archive"; then
    echo "release-npm: $package_name $npm_release_version is already public with matching bytes; skipping"
    continue
  else
    status=$?
    if (( status != 1 )); then
      exit "$status"
    fi
  fi

  if (( publish == 0 )); then
    echo "release-npm: $package_name $npm_release_version is ready to publish"
    continue
  fi

  npm publish "$archive" --access public
  wait_for_package "$package_name" "$registry_name" "$archive"
  echo "release-npm: published $package_name $npm_release_version"
done
