// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drives the real egui UI headlessly with egui_kittest: focus a field,
//! select its text and type, through egui's own input events and AccessKit.
//! No window, no keyboard or mouse of the machine running the test.

use eframe::egui::accesskit::Role;
use eframe::egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oc_ui_egui::App;

fn replace_text(harness: &mut Harness<'_, App>, label: &str, text: &str) {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .type_text(text);
    harness.run();
}

#[test]
fn editing_updates_the_cut_list_and_rejects_invalid_input() {
    let mut harness = Harness::new_eframe(|_cc| App::new());
    harness.run();
    assert_eq!(harness.state().view().cut_list[2].length, "28-19/32\"");

    replace_text(&mut harness, "Width (in)", "36");
    let view = harness.state().view();
    assert_eq!(
        view.cut_list[2].length, "34-19/32\"",
        "bottom length after editing"
    );
    assert_eq!(
        view.cut_list[6].width, "17-7/8\"",
        "door width after editing"
    );
    assert_eq!(harness.state().error(), None);

    replace_text(&mut harness, "Width (in)", "thirty");
    assert!(harness.state().error().is_some_and(|e| e.contains("Width")));
    assert_eq!(harness.state().view().cut_list[2].length, "34-19/32\"");
}
