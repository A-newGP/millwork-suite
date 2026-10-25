#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 OpenCabinet contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Measures a UI prototype: executable size, time from launch to the first
# presented frame (median of 5 runs) and memory of the whole process tree
# (Tauri's web view runs in child processes, which count too).
# Usage: measure-ui.sh <label> <executable> <build-seconds>
# Runs only in CI: it launches the application's window.

set -euo pipefail

label=$1
exe=$2
build_seconds=$3
runs=5
timeout_ms=60000

# Prints the resident memory, in KB, of a process and all its descendants.
tree_memory_kb() {
  local root=$1
  if [[ "${RUNNER_OS:-}" == Windows ]]; then
    local winpid
    winpid=$(cat "/proc/$root/winpid")
    powershell -NoProfile -Command "
      \$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, WorkingSetSize
      \$tree = @($winpid); \$i = 0
      while (\$i -lt \$tree.Count) { \$tree += @(\$all | Where-Object ParentProcessId -eq \$tree[\$i] | ForEach-Object ProcessId); \$i++ }
      [int]((\$all | Where-Object { \$tree -contains \$_.ProcessId } | Measure-Object WorkingSetSize -Sum).Sum / 1KB)"
  else
    ps -e -o pid=,ppid=,rss= | awk -v root="$root" '
      { parent[$1] = $2; rss[$1] = $3 }
      END {
        tree[root] = 1; changed = 1
        while (changed) { changed = 0; for (p in parent) if (!(p in tree) && (parent[p] in tree)) { tree[p] = 1; changed = 1 } }
        for (p in tree) total += rss[p]; print total
      }'
  fi
}

now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

times=()
memory_kb=""
for run in $(seq "$runs"); do
  log=$(mktemp)
  start=$(now_ms)
  OC_MEASURE=1 "$exe" > "$log" 2>&1 &
  pid=$!
  until grep -q FIRST_FRAME "$log"; do
    if (( $(now_ms) - start > timeout_ms )); then
      echo "::error::$label did not present a frame within ${timeout_ms} ms"; cat "$log"; kill "$pid" || true; exit 1
    fi
    sleep 0.01
  done
  times+=( $(( $(now_ms) - start )) )
  if (( run == 1 )); then
    sleep 1
    memory_kb=$(tree_memory_kb "$pid")
  fi
  wait "$pid" || true  # the application exits by itself after the marker
done

median_ms=$(printf '%s\n' "${times[@]}" | sort -n | sed -n "$(( (runs + 1) / 2 ))p")
size_kb=$(( $(wc -c < "$exe") / 1024 ))

table="| Prototype | OS | Executable | Launch to first frame (median of $runs) | Memory, process tree | Release build |
| --- | --- | --- | --- | --- | --- |
| $label | ${RUNNER_OS:-local} | ${size_kb} KB | ${median_ms} ms | $(( memory_kb / 1024 )) MB | ${build_seconds} s |"

echo "$table"
echo "all launch times (ms): ${times[*]}"
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  printf '%s\n\n' "$table" >> "$GITHUB_STEP_SUMMARY"
fi
