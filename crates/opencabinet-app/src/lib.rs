// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! OpenCabinet Designer's window, as a library so tests can drive it
//! without a window system (see `tests/window.rs`).

pub mod designer;
pub mod dialogs;
pub mod drawing;
pub mod fonts;
pub mod perf;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use opencabinet_model::Units;
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, StandardListViewItem, VecModel};

use crate::designer::{CabinetKind, Designer, Dimension};
use crate::dialogs::{Dialogs, FileKind, SystemDialogs};
use opencabinet_reports::Separator;

/// The code Slint generates from `ui/app.slint`, which follows Slint's
/// conventions rather than this workspace's lints.
#[allow(missing_docs, clippy::pedantic)]
mod ui {
    slint::include_modules!();
}

pub use ui::AppWindow;

/// The name and version shown to users and written into saved files.
pub const NAME_AND_VERSION: &str = concat!("OpenCabinet Designer ", env!("CARGO_PKG_VERSION"));

/// Creates the window, asking the user through the system's dialogs.
///
/// # Errors
/// When the platform cannot create a window.
pub fn create() -> Result<AppWindow, slint::PlatformError> {
    create_with(Rc::new(SystemDialogs))
}

/// Creates the window and connects it to a new [`Designer`], asking the
/// user through `dialogs`.
///
/// # Errors
/// When the platform cannot create a window.
pub fn create_with(dialogs: Rc<dyn Dialogs>) -> Result<AppWindow, slint::PlatformError> {
    let window = AppWindow::new()?;
    let app = Rc::new(App {
        window: window.as_weak(),
        designer: RefCell::new(Designer::default()),
        dialogs,
    });
    app.show(&window, true);

    for dimension in Dimension::ALL {
        let app = Rc::clone(&app);
        let handler = move |text: slint::SharedString| app.edit(dimension, &text);
        match dimension {
            Dimension::Width => window.on_width_edited(handler),
            Dimension::Height => window.on_height_edited(handler),
            Dimension::Depth => window.on_depth_edited(handler),
        }
    }
    let handle = Rc::clone(&app);
    window.on_units_chosen(move |metric| handle.choose_units(metric));
    let handle = Rc::clone(&app);
    window.on_cabinet_chosen(move |index| handle.choose_cabinet(index));
    let handle = Rc::clone(&app);
    window.on_add_cabinet(move || {
        handle.change_cabinets(|d| {
            d.add();
            Ok(())
        });
    });
    let handle = Rc::clone(&app);
    window.on_duplicate_cabinet(move || {
        handle.change_cabinets(|d| {
            d.duplicate();
            Ok(())
        });
    });
    let handle = Rc::clone(&app);
    window.on_delete_cabinet(move || handle.change_cabinets(Designer::delete));
    let handle = Rc::clone(&app);
    window.on_label_edited(move |text| handle.edit_label(&text));
    window.set_kinds(ModelRc::new(VecModel::from(
        CabinetKind::ALL
            .map(|k| slint::SharedString::from(k.name()))
            .to_vec(),
    )));
    let handle = Rc::clone(&app);
    window.on_kind_chosen(move |index| handle.choose_kind(index));
    let handle = Rc::clone(&app);
    window.on_style_chosen(move |five_piece| handle.choose_style(five_piece));
    let handle = Rc::clone(&app);
    window.on_new_project(move || handle.new_project());
    let handle = Rc::clone(&app);
    window.on_open_project(move || handle.open());
    let handle = Rc::clone(&app);
    window.on_save_project(move || {
        handle.save(false);
    });
    let handle = Rc::clone(&app);
    window.on_save_project_as(move || {
        handle.save(true);
    });
    let handle = Rc::clone(&app);
    window.on_export_cut_list(move |semicolon| {
        let separator = if semicolon {
            Separator::Semicolon
        } else {
            Separator::Comma
        };
        handle.export_cut_list(separator);
    });
    let handle = Rc::clone(&app);
    window.on_export_cut_list_pdf(move || handle.export_cut_list_pdf());
    let handle = Rc::clone(&app);
    window.on_export_parts_dxf(move || handle.export_parts_dxf());
    let handle = Rc::clone(&app);
    window.on_about(move || {
        handle.status(&format!(
            "{NAME_AND_VERSION} — free software under the GNU GPL, version 3 or later"
        ));
    });
    let handle = Rc::clone(&app);
    window.on_quit(move || {
        if handle.may_discard() {
            // Fails only when no event loop runs, as in tests: nothing to stop.
            slint::quit_event_loop().ok();
        }
    });
    let handle = Rc::clone(&app);
    window.window().on_close_requested(move || {
        if handle.may_discard() {
            CloseRequestResponse::HideWindow
        } else {
            CloseRequestResponse::KeepWindowShown
        }
    });
    Ok(window)
}

/// The window's state and what it does with it.
struct App {
    window: slint::Weak<AppWindow>,
    designer: RefCell<Designer>,
    dialogs: Rc<dyn Dialogs>,
}

impl App {
    fn edit(&self, dimension: Dimension, text: &str) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let result = self.designer.borrow_mut().edit(dimension, text);
        set_error(&window, dimension, result.err().unwrap_or_default());
        // Keep what the user is typing; only the readings change.
        self.show(&window, false);
    }

    fn choose_units(&self, metric: bool) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let units = if metric {
            Units::Metric
        } else {
            Units::Imperial
        };
        self.designer.borrow_mut().set_units(units);
        clear_errors(&window);
        self.show(&window, true);
    }

    fn choose_cabinet(&self, index: i32) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let chosen = usize::try_from(index).is_ok_and(|i| self.designer.borrow_mut().select(i));
        if chosen {
            clear_errors(&window);
            self.show(&window, true);
        }
    }

    /// Adds, copies or removes a cabinet with `change`, then shows the one
    /// to edit, or says why it could not.
    fn change_cabinets(&self, change: impl FnOnce(&mut Designer) -> Result<(), String>) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let result = change(&mut self.designer.borrow_mut());
        clear_errors(&window);
        self.show(&window, true);
        if let Err(message) = result {
            self.status(&message);
        }
    }

    fn edit_label(&self, text: &str) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let result = self.designer.borrow_mut().set_label(text);
        window.set_label_error(result.err().unwrap_or_default().into());
        self.show(&window, false);
    }

    fn choose_kind(&self, index: i32) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let Some(&kind) = usize::try_from(index)
            .ok()
            .and_then(|i| CabinetKind::ALL.get(i))
        else {
            return;
        };
        self.designer.borrow_mut().set_kind(kind);
        clear_errors(&window);
        self.show(&window, true);
    }

    fn choose_style(&self, five_piece: bool) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        self.designer.borrow_mut().set_five_piece(five_piece);
        self.show(&window, false);
    }

    fn new_project(&self) {
        if !self.may_discard() {
            return;
        }
        let units = self.designer.borrow().units();
        self.replace(Designer::new(units));
    }

    fn open(&self) {
        if !self.may_discard() {
            return;
        }
        let Some(path) = self.dialogs.open_path() else {
            return;
        };
        match Designer::open(&path) {
            Ok(designer) => {
                let problems = designer.issues().len();
                self.replace(designer);
                let opened = format!("Opened {}", file_name(&path));
                let message = match problems {
                    0 => opened,
                    n => format!("{opened} · {n} problems found"),
                };
                self.status(&message);
            }
            Err(message) => self.status(&message),
        }
    }

    /// Saves to the project's file, or asks where when it has none or
    /// `ask` is set. Returns whether the project was saved.
    fn save(&self, ask: bool) -> bool {
        let known = self.designer.borrow().path().map(Path::to_owned);
        let path = match known {
            Some(path) if !ask => path,
            _ => {
                let suggested = format!("{}.ocab", self.designer.borrow().label());
                let Some(path) = self.dialogs.save_path(&suggested, FileKind::Project) else {
                    return false;
                };
                path
            }
        };
        let result = self.designer.borrow_mut().save_as(&path);
        let Some(window) = self.window.upgrade() else {
            return false;
        };
        self.show(&window, false);
        match result {
            Ok(()) => {
                self.status(&format!("Saved {}", file_name(&path)));
                true
            }
            Err(message) => {
                self.status(&message);
                false
            }
        }
    }

    fn export_cut_list(&self, separator: Separator) {
        let suggested = format!("{} cut list.csv", self.designer.borrow().label());
        let Some(path) = self.dialogs.save_path(&suggested, FileKind::CutList) else {
            return;
        };
        match self.designer.borrow().export_cut_list(&path, separator) {
            Ok(()) => self.status(&format!("Exported the cut list to {}", file_name(&path))),
            Err(message) => self.status(&message),
        }
    }

    fn export_cut_list_pdf(&self) {
        let fonts = match fonts::load() {
            Ok(fonts) => fonts,
            Err(message) => return self.status(&message),
        };
        let suggested = format!("{} cut list.pdf", self.designer.borrow().label());
        let Some(path) = self.dialogs.save_path(&suggested, FileKind::CutListPdf) else {
            return;
        };
        match self.designer.borrow().export_cut_list_pdf(&path, &fonts) {
            Ok(()) => self.status(&format!("Exported the cut list to {}", file_name(&path))),
            Err(message) => self.status(&message),
        }
    }

    fn export_parts_dxf(&self) {
        let Some(folder) = self.dialogs.dxf_folder() else {
            return;
        };
        match self.designer.borrow().export_parts_dxf(&folder) {
            Ok(parts) => self.status(&format!(
                "Exported {parts} parts as DXF to {}",
                file_name(&folder)
            )),
            Err(message) => self.status(&message),
        }
    }

    /// Whether the current project may be closed: it has no unsaved
    /// changes, or the user agrees to lose them.
    fn may_discard(&self) -> bool {
        !self.designer.borrow().modified() || self.dialogs.discard_changes()
    }

    fn replace(&self, designer: Designer) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        *self.designer.borrow_mut() = designer;
        clear_errors(&window);
        self.show(&window, true);
    }

    /// Shows the designer's state; with `texts`, also rewrites the fields.
    fn show(&self, window: &AppWindow, texts: bool) {
        let designer = self.designer.borrow();
        if texts {
            window.set_label_text(designer.label().into());
            window.set_width_text(designer.text(Dimension::Width).into());
            window.set_height_text(designer.text(Dimension::Height).into());
            window.set_depth_text(designer.text(Dimension::Depth).into());
        }
        window.set_width_reading(designer.reading(Dimension::Width).into());
        window.set_height_reading(designer.reading(Dimension::Height).into());
        window.set_depth_reading(designer.reading(Dimension::Depth).into());
        show_drawing(window, &designer);
        let list = designer.cut_list();
        let rows: Vec<ModelRc<StandardListViewItem>> = list
            .rows
            .iter()
            .map(|row| {
                let cells: Vec<StandardListViewItem> = row
                    .cells()
                    .into_iter()
                    .map(StandardListViewItem::from)
                    .collect();
                ModelRc::new(VecModel::from(cells))
            })
            .collect();
        window.set_cut_rows(ModelRc::new(VecModel::from(rows)));
        window.set_cut_problems(list.problems.join("\n").into());
        let cabinets: Vec<StandardListViewItem> = designer
            .cabinets()
            .iter()
            .map(|c| StandardListViewItem::from(c.as_str()))
            .collect();
        window.set_cabinets(ModelRc::new(VecModel::from(cabinets)));
        window.set_cabinet_index(i32::try_from(designer.selected()).unwrap_or(0));
        window.set_window_title(designer.title().into());
        window.set_cabinet_label(designer.label().into());
        let kind = designer
            .kind()
            .and_then(|k| CabinetKind::ALL.iter().position(|&c| c == k));
        window.set_kind_index(kind.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1));
        window.set_style_index(i32::from(designer.five_piece()));
        let metric = designer.units() == Units::Metric;
        window.set_metric(metric);
        let units = if metric { "millimeters" } else { "inches" };
        window.set_status(format!("{NAME_AND_VERSION} · {units}").into());
    }

    fn status(&self, message: &str) {
        if let Some(window) = self.window.upgrade() {
            window.set_status(message.into());
        }
    }
}

/// The drawing area: the elevation or the cabinet, and what is wrong with
/// the placements.
fn show_drawing(window: &AppWindow, designer: &Designer) {
    let drawing = designer.drawing();
    // An empty path is not drawn; a move alone draws nothing.
    let path = |commands: String| {
        if commands.is_empty() {
            "M 0 0".into()
        } else {
            commands.into()
        }
    };
    window.set_drawing_title(drawing.title.into());
    window.set_drawing_width(drawing.width);
    window.set_drawing_height(drawing.height);
    window.set_drawing_lines(path(drawing.lines));
    window.set_drawing_marks(path(drawing.marks));
    window.set_drawing_highlight(path(drawing.highlight));
    window.set_drawing_dimensions(path(drawing.dimensions));
    let labels: Vec<ui::DrawingLabel> = drawing
        .labels
        .into_iter()
        .map(|l| ui::DrawingLabel {
            x: l.x,
            y: l.y,
            text: l.text.into(),
        })
        .collect();
    window.set_drawing_labels(ModelRc::new(VecModel::from(labels)));
    window.set_placement_problems(designer.placement_problems().join("\n").into());
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
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
    window.set_label_error(slint::SharedString::new());
}
