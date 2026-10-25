// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drives the real Slint UI through its accessibility tree, as a screen
//! reader would, using Slint's headless testing backend: no window, no
//! keyboard or mouse of the machine running the test.
//!
//! The backend can be initialized once per process, so the scenarios run in
//! a single test, in order.

use i_slint_backend_testing::ElementHandle;
use slint::Model;

fn field(app: &oc_ui_slint::AppWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(app, label)
        .find(|e| e.accessible_value().is_some())
        .unwrap_or_else(|| panic!("no editable field labelled {label}"))
}

fn cell(app: &oc_ui_slint::AppWindow, row: usize, column: usize) -> String {
    let row = app.get_rows().row_data(row).expect("row exists");
    row.row_data(column).expect("cell exists").text.to_string()
}

#[test]
fn editing_updates_the_cut_list_and_rejects_invalid_input() {
    i_slint_backend_testing::init_integration_test_with_system_time();
    let app = oc_ui_slint::create().unwrap();
    assert_eq!(
        cell(&app, 2, 3),
        "28-19/32\"",
        "bottom length before editing"
    );

    // A valid width updates every dependent part.
    field(&app, "Width (in)").set_accessible_value("36");
    assert_eq!(
        cell(&app, 2, 3),
        "34-19/32\"",
        "bottom length after editing"
    );
    assert_eq!(cell(&app, 6, 4), "17-7/8\"", "door width after editing");
    assert!(app.get_error().is_empty());

    // Invalid input shows an error and keeps the last valid result.
    field(&app, "Width (in)").set_accessible_value("thirty");
    assert!(
        app.get_error().contains("Width"),
        "error: {}",
        app.get_error()
    );
    assert_eq!(cell(&app, 2, 3), "34-19/32\"");
}
