// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drives the real window through its accessibility tree, as a screen
//! reader would, with Slint's headless testing backend: no screen, keyboard
//! or mouse of the machine running the test is used.
//!
//! The backend can start once per process, so the scenarios run in order in
//! a single test.

use i_slint_backend_testing::ElementHandle;
use opencabinet_app::AppWindow;

fn field(window: &AppWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(window, label)
        .find(|e| e.accessible_value().is_some())
        .unwrap_or_else(|| panic!("no editable field labelled {label}"))
}

#[test]
fn the_window_reads_lengths_switches_units_and_starts_over() {
    i_slint_backend_testing::init_integration_test_with_system_time();
    let window = opencabinet_app::create().unwrap();
    assert_eq!(window.get_width_text(), "30\"");
    assert!(window.get_status().contains("inches"));
    assert_eq!(
        ElementHandle::find_by_accessible_label(&window, "Cabinet front").count(),
        1
    );

    // Shop notation is read exactly and shown back in lowest terms.
    field(&window, "Width").set_accessible_value("30 1/2");
    assert_eq!(window.get_width_reading(), "= 30-1/2\"");
    assert!(window.get_width_error().is_empty());

    // Something that is not a length explains why and keeps the last value.
    field(&window, "Height").set_accessible_value("tall");
    assert_eq!(window.get_height_error(), "Not a length");
    assert_eq!(window.get_height_reading(), "= 34-1/2\"");

    // Units change what is shown, never what is stored.
    window.invoke_units_chosen(true);
    assert!(window.get_metric());
    assert_eq!(window.get_width_text(), "774.7 mm");
    assert!(window.get_height_error().is_empty());
    field(&window, "Depth").set_accessible_value("600");
    window.invoke_units_chosen(false);
    assert_eq!(window.get_depth_reading(), "≈ 23-5/8\"");

    // A new project starts from the defaults, in the units chosen.
    window.invoke_new_project();
    assert_eq!(window.get_width_text(), "30\"");

    window.invoke_about();
    assert!(window.get_status().contains("GNU GPL"));
}
