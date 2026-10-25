// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#![windows_subsystem = "windows"]

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenCabinet UI spike — egui")
            .with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "oc-ui-egui",
        options,
        Box::new(|_cc| Ok(Box::new(oc_ui_egui::App::new()))),
    )
}
