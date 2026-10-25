// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#![windows_subsystem = "windows"]

use slint::{ComponentHandle, RenderingState};

fn main() -> Result<(), slint::PlatformError> {
    let app = oc_ui_slint::create()?;

    let mut reported = false;
    app.window()
        .set_rendering_notifier(move |state, _| {
            if matches!(state, RenderingState::AfterRendering) && !reported {
                reported = true;
                oc_ui_core::first_frame_presented();
            }
        })
        .ok();

    app.run()
}
