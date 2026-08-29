#!/usr/bin/env bash

set -euo pipefail

if [[ $(uname -s) != Darwin ]]; then
  echo "macos-acceptance-gate: this live AppKit/WindowServer probe requires macOS" >&2
  exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
cd "$repository_root"

target_dir=${CARGO_TARGET_DIR:-"$repository_root/target"}
if [[ $target_dir != /* ]]; then
  target_dir="$repository_root/$target_dir"
fi
binary="$target_dir/release/examples/macos_acceptance"

max_process_cpu_percent=${QUICKGUI_ACCEPTANCE_MAX_PROCESS_CPU_PERCENT:-35}
max_rss_mib=${QUICKGUI_ACCEPTANCE_MAX_RSS_MIB:-192}
max_footprint_mib=${QUICKGUI_ACCEPTANCE_MAX_FOOTPRINT_MIB:-320}

probe_log=$(mktemp -t quickgui-acceptance-probe.XXXXXX)
time_log=$(mktemp -t quickgui-acceptance-time.XXXXXX)
trap 'rm -f "$probe_log" "$time_log"' EXIT

cargo build --release --example macos_acceptance --locked

set +e
/usr/bin/time -l -o "$time_log" "$binary" 2>&1 | tee "$probe_log"
probe_status=${PIPESTATUS[0]}
set -e

cat "$time_log"

read -r real_seconds user_seconds system_seconds < <(
  awk '/ real .* user .* sys$/ { print $1, $3, $5; exit }' "$time_log"
)
max_rss_bytes=$(awk '/ maximum resident set size$/ { print $1; exit }' "$time_log")
peak_footprint_bytes=$(awk '/ peak memory footprint$/ { print $1; exit }' "$time_log")

if [[ -z ${real_seconds:-} || -z ${user_seconds:-} || -z ${system_seconds:-} || -z ${max_rss_bytes:-} || -z ${peak_footprint_bytes:-} ]]; then
  echo "macos-acceptance-gate: could not parse /usr/bin/time output" >&2
  exit 1
fi

process_cpu_percent=$(awk \
  -v real="$real_seconds" \
  -v user="$user_seconds" \
  -v sys_cpu="$system_seconds" \
  'BEGIN { if (real <= 0) print "0.000"; else printf "%.3f", (user + sys_cpu) / real * 100 }')
max_rss_observed_mib=$(awk \
  -v bytes="$max_rss_bytes" \
  'BEGIN { printf "%.3f", bytes / 1024 / 1024 }')
peak_footprint_observed_mib=$(awk \
  -v bytes="$peak_footprint_bytes" \
  'BEGIN { printf "%.3f", bytes / 1024 / 1024 }')

process_passed=true
if awk -v actual="$process_cpu_percent" -v maximum="$max_process_cpu_percent" \
  'BEGIN { exit !(actual > maximum) }'; then
  echo "macos-acceptance-gate: process CPU ${process_cpu_percent}% exceeds ${max_process_cpu_percent}%" >&2
  process_passed=false
fi
if awk -v actual="$max_rss_observed_mib" -v maximum="$max_rss_mib" \
  'BEGIN { exit !(actual > maximum) }'; then
  echo "macos-acceptance-gate: peak RSS ${max_rss_observed_mib} MiB exceeds ${max_rss_mib} MiB" >&2
  process_passed=false
fi
if awk -v actual="$peak_footprint_observed_mib" -v maximum="$max_footprint_mib" \
  'BEGIN { exit !(actual > maximum) }'; then
  echo "macos-acceptance-gate: peak physical footprint ${peak_footprint_observed_mib} MiB exceeds ${max_footprint_mib} MiB" >&2
  process_passed=false
fi

printf '%s\n' \
  "QUICKGUI_ACCEPTANCE_PROCESS_RESULT {\"process_cpu_percent\":${process_cpu_percent},\"max_rss_mib\":${max_rss_observed_mib},\"peak_footprint_mib\":${peak_footprint_observed_mib},\"max_process_cpu_percent\":${max_process_cpu_percent},\"max_allowed_rss_mib\":${max_rss_mib},\"max_allowed_footprint_mib\":${max_footprint_mib},\"passed\":${process_passed}}"

if [[ $probe_status -ne 0 ]]; then
  echo "macos-acceptance-gate: in-app probe exited with status $probe_status" >&2
  exit "$probe_status"
fi
if ! grep -q '^QUICKGUI_ACCEPTANCE_RESULT .*"passed":true' "$probe_log"; then
  echo "macos-acceptance-gate: no passing resize/text/native-view/focus/document/popover result was emitted" >&2
  exit 1
fi
if ! grep -q '^QUICKGUI_LIFECYCLE_RESULT .*"passed":true' "$probe_log"; then
  echo "macos-acceptance-gate: no passing native-view teardown result was emitted" >&2
  exit 1
fi
if [[ $process_passed != true ]]; then
  exit 1
fi
