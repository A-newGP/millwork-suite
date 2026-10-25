#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 OpenCabinet contributors
# SPDX-License-Identifier: GPL-3.0-or-later

# Prints a Markdown table per exporter: code size, dependencies, executable
# size and run time. Run from prototypes/export-spike after a release build.
set -euo pipefail

exe_suffix=""
[[ "${OS:-}" == "Windows_NT" ]] && exe_suffix=".exe"
out=$(mktemp -d)

echo "| Exporter | Formats | Lines of code | Crates | Executable | Run (median of 5) |"
echo "| --- | --- | --- | --- | --- | --- |"
for name in plain dxf krilla printpdf; do
  case $name in
    plain) formats="DXF R12, CSV" ;;
    dxf) formats="DXF 2018" ;;
    *) formats="PDF" ;;
  esac
  # Non-blank lines that are not comments.
  lines=$(grep -cvE '^\s*(//|$)' "$name/src/main.rs")
  crates=$(cargo tree --locked -p "oc-export-$name" -e normal --prefix none | sort -u | wc -l)
  exe="target/release/oc-export-$name$exe_suffix"
  size=$(( $(wc -c < "$exe") / 1024 ))
  times=()
  for _ in 1 2 3 4 5; do
    start=$(date +%s%N)
    "$exe" "$out" >/dev/null
    times+=($(( ($(date +%s%N) - start) / 1000000 )))
  done
  median=$(printf '%s\n' "${times[@]}" | sort -n | sed -n 3p)
  echo "| $name | $formats | $lines | $crates | $size KB | $median ms |"
done
rm -rf "$out"
