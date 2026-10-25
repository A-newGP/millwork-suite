#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 OpenCabinet contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# System libraries the three front ends need on Ubuntu, installed in every
# Linux job so the comparison is equal. Tauri: WebKitGTK and GTK; Slint and
# egui: fontconfig, xkbcommon and Mesa OpenGL; xvfb for the measurements.

set -euo pipefail

sudo apt-get update -q
sudo apt-get install -yq --no-install-recommends \
  libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev librsvg2-dev \
  libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev \
  libgl1-mesa-dri xvfb
