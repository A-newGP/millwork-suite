#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 OpenCabinet contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Verifies a prototype executable against the golden outputs and measures it.
# Usage: measure.sh <label> <executable> <build-seconds>
# Appends a Markdown table to $GITHUB_STEP_SUMMARY when it is set.

set -euo pipefail

label=$1
exe=$2
build_seconds=$3
spike=$(cd "$(dirname "$0")" && pwd)
fixture="$spike/fixtures/sample.ocab"
out=$(mktemp -d)

# Correctness: byte-exact golden outputs and a lossless round trip.
"$exe" evaluate "$fixture" "$out"
git diff --no-index --exit-code "$spike/expected/cutlist.csv" "$out/cutlist.csv"
git diff --no-index --exit-code "$spike/expected/operations.csv" "$out/operations.csv"
"$exe" roundtrip "$fixture" "$out/roundtrip.ocab"

size_kb=$(( $(wc -c < "$exe") / 1024 ))

# Start-up plus evaluation, as a user experiences it: mean of 20 runs.
runs=20
start=$(date +%s%N)
for _ in $(seq "$runs"); do
  "$exe" evaluate "$fixture" "$out"
done
end=$(date +%s%N)
run_ms=$(( (end - start) / runs / 1000000 ))

# Evaluation alone, in-process.
eval_ns=$("$exe" bench "$fixture" 100000 | cut -d' ' -f1)

if [[ -x /usr/bin/time ]]; then
  peak_kb=$( { /usr/bin/time -f '%M' "$exe" evaluate "$fixture" "$out" >/dev/null; } 2>&1 | tail -n1 )
else
  peak_kb="n/a"
fi

table="| Prototype | OS | Executable | Run (start-up + evaluate) | Evaluate only | Peak memory | Release build |
| --- | --- | --- | --- | --- | --- | --- |
| $label | ${RUNNER_OS:-local} | ${size_kb} KB | ${run_ms} ms | ${eval_ns} ns | ${peak_kb} KB | ${build_seconds} s |"

echo "$table"
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  printf '%s\n\n' "$table" >> "$GITHUB_STEP_SUMMARY"
fi
