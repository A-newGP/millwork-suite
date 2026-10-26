// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drives the real window through its accessibility tree, as a screen
//! reader would, with Slint's headless testing backend: no screen, keyboard
//! or mouse of the machine running the test is used, and the dialogs give
//! fixed answers instead of opening.
//!
//! The backend can start once per process, so the scenarios run in order in
//! a single test.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use opencabinet_app::AppWindow;
use opencabinet_app::dialogs::{Dialogs, FileKind};
use slint::Model;

/// Answers the dialogs from the test, and counts the questions.
#[derive(Default)]
struct Answers {
    path: RefCell<Option<PathBuf>>,
    discard: Cell<bool>,
    asked_to_discard: Cell<u32>,
}

impl Dialogs for Answers {
    fn open_path(&self) -> Option<PathBuf> {
        self.path.borrow().clone()
    }

    fn save_path(&self, _suggested_name: &str, kind: FileKind) -> Option<PathBuf> {
        let extension = match kind {
            FileKind::Project => "ocab",
            FileKind::CutList => "csv",
            FileKind::CutListPdf => "pdf",
        };
        self.path
            .borrow()
            .as_ref()
            .map(|p| p.with_extension(extension))
    }

    fn discard_changes(&self) -> bool {
        self.asked_to_discard.set(self.asked_to_discard.get() + 1);
        self.discard.get()
    }
}

fn field(window: &AppWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(window, label)
        .find(|e| e.accessible_value().is_some())
        .unwrap_or_else(|| panic!("no editable field labelled {label}"))
}

#[test]
fn the_window_edits_saves_and_reopens_a_project() {
    i_slint_backend_testing::init_integration_test_with_system_time();
    let answers = Rc::new(Answers::default());
    let window = opencabinet_app::create_with(answers.clone()).unwrap();
    assert_eq!(window.get_width_text(), "30\"");
    assert_eq!(window.get_window_title(), "Untitled — OpenCabinet Designer");
    assert_eq!(
        ElementHandle::find_by_accessible_label(&window, "Cabinet front").count(),
        1
    );

    // The cut list shows the starter cabinet's fourteen parts, drawer box
    // included, with nothing left out.
    let cell = |row: usize, column: usize| {
        let row = window.get_cut_rows().row_data(row).unwrap();
        row.row_data(column).unwrap().text.to_string()
    };
    assert_eq!(window.get_cut_rows().row_count(), 14);
    assert_eq!(
        (cell(2, 1), cell(2, 4)),
        ("Bottom".into(), "28-19/32\"".into())
    );
    assert_eq!(window.get_cut_problems(), "");

    // Shop notation is read exactly and shown back in lowest terms.
    field(&window, "Width").set_accessible_value("30 1/2");
    assert_eq!(window.get_width_reading(), "= 30-1/2\"");
    assert_eq!(window.get_width_error(), "");
    // The cut list follows the new width at once.
    assert_eq!(cell(2, 4), "29-3/32\"");
    assert_eq!(
        window.get_window_title(),
        "Untitled* — OpenCabinet Designer"
    );

    // Something that is not a length explains why and keeps the last value.
    field(&window, "Height").set_accessible_value("tall");
    assert_eq!(window.get_height_error(), "Not a length");
    assert_eq!(window.get_height_reading(), "= 34-1/2\"");

    // Units change what is shown, never what is stored.
    window.invoke_units_chosen(true);
    assert!(window.get_metric());
    assert_eq!(window.get_width_text(), "774.7 mm");
    assert_eq!(window.get_height_error(), "");
    field(&window, "Depth").set_accessible_value("600");
    window.invoke_units_chosen(false);
    assert_eq!(window.get_depth_reading(), "≈ 23-5/8\"");

    // Refusing to discard changes keeps the project.
    window.invoke_new_project();
    assert_eq!(answers.asked_to_discard.get(), 1);
    assert_eq!(window.get_width_text(), "30-1/2\"");

    // Saving writes the file the dialog chose and clears the change mark.
    let dir = std::env::temp_dir().join(format!("opencabinet-window-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test-cabinet.ocab");
    *answers.path.borrow_mut() = Some(path.clone());
    window.invoke_save_project();
    assert!(path.is_file());
    assert_eq!(
        window.get_window_title(),
        "test-cabinet.ocab — OpenCabinet Designer"
    );
    assert_eq!(window.get_status(), "Saved test-cabinet.ocab");

    // A saved project starts over without asking, and reopens as saved.
    window.invoke_new_project();
    assert_eq!(answers.asked_to_discard.get(), 1);
    assert_eq!(window.get_width_text(), "30\"");
    window.invoke_open_project();
    assert_eq!(window.get_width_text(), "30-1/2\"");
    assert_eq!(window.get_depth_reading(), "≈ 23-5/8\"");
    assert_eq!(window.get_status(), "Opened test-cabinet.ocab");

    // The cut list exports for Excel in Spanish: UTF-8 with a byte order
    // mark, semicolons between fields.
    window.invoke_export_cut_list(true);
    let csv = std::fs::read_to_string(path.with_extension("csv")).unwrap();
    assert!(csv.starts_with("\u{feff}Product;Part;Material;"), "{csv}");
    assert_eq!(csv.lines().count(), 15);
    assert_eq!(
        window.get_status(),
        "Exported the cut list to test-cabinet.csv"
    );

    // And as a PDF, with the bundled font.
    window.invoke_export_cut_list_pdf();
    let pdf = std::fs::read(path.with_extension("pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(
        window.get_status(),
        "Exported the cut list to test-cabinet.pdf"
    );

    window.invoke_about();
    assert!(window.get_status().contains("GNU GPL"));
}
