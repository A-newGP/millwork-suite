// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! OpenCabinet Designer's window, as a library so tests can drive it
//! without a window system (see `tests/window.rs`).

pub mod designer;
pub mod perf;

use std::cell::RefCell;
use std::rc::Rc;

use opencabinet_model::Units;
use slint::ComponentHandle;

use crate::designer::{Designer, Dimension};

/// The code Slint generates from `ui/app.slint`, which follows Slint's
/// conventions rather than this workspace's lints.
#[allow(missing_docs, clippy::pedantic)]
mod ui {
    slint::include_modules!();
}

pub use ui::AppWindow;

/// The name and version shown to users.
pub const NAME_AND_VERSION: &str = concat!("OpenCabinet Designer ", env!("CARGO_PKG_VERSION"));

/// Creates the window and connects it to a new [`Designer`].
///
/// # Errors
/// When the platform cannot create a window.
pub fn create() -> Result<AppWindow, slint::PlatformError> {
    let window = AppWindow::new()?;
    let designer = Rc::new(RefCell::new(Designer::default()));
    refresh(&window, &designer.borrow(), true);

    for dimension in Dimension::ALL {
        let (weak, designer) = (window.as_weak(), Rc::clone(&designer));
        let handler = move |text: slint::SharedString| {
            let Some(window) = weak.upgrade() else { return };
            let result = designer.borrow_mut().edit(dimension, &text);
            set_error(&window, dimension, result.err().unwrap_or_default());
            // Keep what the user is typing; only the readings change.
            refresh(&window, &designer.borrow(), false);
        };
        match dimension {
            Dimension::Width => window.on_width_edited(handler),
            Dimension::Height => window.on_height_edited(handler),
            Dimension::Depth => window.on_depth_edited(handler),
        }
    }

    let (weak, state) = (window.as_weak(), Rc::clone(&designer));
    window.on_units_chosen(move |metric| {
        let Some(window) = weak.upgrade() else { return };
        let units = if metric {
            Units::Metric
        } else {
            Units::Imperial
        };
        state.borrow_mut().set_units(units);
        clear_errors(&window);
        refresh(&window, &state.borrow(), true);
    });

    let (weak, state) = (window.as_weak(), Rc::clone(&designer));
    window.on_new_project(move || {
        let Some(window) = weak.upgrade() else { return };
        let units = state.borrow().units();
        let mut fresh = Designer::default();
        fresh.set_units(units);
        *state.borrow_mut() = fresh;
        clear_errors(&window);
        refresh(&window, &state.borrow(), true);
    });

    let weak = window.as_weak();
    window.on_about(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status(
                format!("{NAME_AND_VERSION} — free software under the GNU GPL, version 3 or later")
                    .into(),
            );
        }
    });
    window.on_quit(|| {
        // Fails only when no event loop runs, as in tests: nothing to stop.
        slint::quit_event_loop().ok();
    });
    Ok(window)
}

/// Shows the designer's state; with `texts`, also rewrites the fields.
fn refresh(window: &AppWindow, designer: &Designer, texts: bool) {
    if texts {
        window.set_width_text(designer.text(Dimension::Width).into());
        window.set_height_text(designer.text(Dimension::Height).into());
        window.set_depth_text(designer.text(Dimension::Depth).into());
    }
    window.set_width_reading(designer.reading(Dimension::Width).into());
    window.set_height_reading(designer.reading(Dimension::Height).into());
    window.set_depth_reading(designer.reading(Dimension::Depth).into());
    window.set_front_ratio(designer.front_ratio());
    let metric = designer.units() == Units::Metric;
    window.set_metric(metric);
    let units = if metric { "millimeters" } else { "inches" };
    window.set_status(format!("{NAME_AND_VERSION} · {units}").into());
}

fn set_error(window: &AppWindow, dimension: Dimension, message: String) {
    let message = message.into();
    match dimension {
        Dimension::Width => window.set_width_error(message),
        Dimension::Height => window.set_height_error(message),
        Dimension::Depth => window.set_depth_error(message),
    }
}

fn clear_errors(window: &AppWindow) {
    for dimension in Dimension::ALL {
        set_error(window, dimension, String::new());
    }
}
