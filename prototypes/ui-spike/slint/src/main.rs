// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#![windows_subsystem = "windows"]

use std::time::Duration;

use slint::{ComponentHandle, RenderingState, Timer, TimerMode};

fn main() -> Result<(), slint::PlatformError> {
    let app = oc_ui_slint::create()?;

    let mut reported = false;
    let notifier = app.window().set_rendering_notifier(move |state, _| {
        if matches!(state, RenderingState::AfterRendering) && !reported {
            reported = true;
            oc_ui_core::first_frame_presented();
        }
    });

    // The software renderer, used when no GPU is available, has no rendering
    // notifier. Fall back to the first timer tick after the window is shown,
    // which runs once the event loop has painted the initial frame.
    let fallback = Timer::default();
    if notifier.is_err() {
        fallback.start(
            TimerMode::SingleShot,
            Duration::from_millis(1),
            oc_ui_core::first_frame_presented,
        );
    }

    app.run()
}
