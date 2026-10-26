// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The questions the window asks the user in the system's own dialogs:
//! which file to open, where to save, and whether to discard changes.
//! Tests replace them with fixed answers.

use std::path::PathBuf;

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

/// Asks the user through dialogs.
pub trait Dialogs {
    /// The project file to open, or `None` when the user cancels.
    fn open_path(&self) -> Option<PathBuf>;

    /// Where to save the project, starting from `suggested_name`, or `None`
    /// when the user cancels.
    fn save_path(&self, suggested_name: &str) -> Option<PathBuf>;

    /// Whether the user agrees to lose the changes that are not saved.
    fn discard_changes(&self) -> bool;
}

/// The operating system's own dialogs.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDialogs;

const FILTER_NAME: &str = "OpenCabinet project";
const EXTENSION: &str = "ocab";

impl Dialogs for SystemDialogs {
    fn open_path(&self) -> Option<PathBuf> {
        FileDialog::new()
            .set_title("Open project")
            .add_filter(FILTER_NAME, &[EXTENSION])
            .pick_file()
    }

    fn save_path(&self, suggested_name: &str) -> Option<PathBuf> {
        let path = FileDialog::new()
            .set_title("Save project")
            .add_filter(FILTER_NAME, &[EXTENSION])
            .set_file_name(suggested_name)
            .save_file()?;
        // Some systems do not add the extension the filter shows.
        Some(if path.extension().is_some_and(|e| e == EXTENSION) {
            path
        } else {
            path.with_extension(EXTENSION)
        })
    }

    fn discard_changes(&self) -> bool {
        let answer = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("Unsaved changes")
            .set_description("The project has changes that are not saved. Discard them?")
            .set_buttons(MessageButtons::YesNo)
            .show();
        answer == MessageDialogResult::Yes
    }
}
