// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Slint front end as a library, so tests can drive it without a window
//! system (see `tests/edit.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use oc_ui_core::drawing::{Shape, ShapeKind};
use oc_ui_core::length::Length;
use oc_ui_core::{Params, Session, View};
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel};

slint::include_modules!();

#[allow(clippy::cast_possible_truncation)]
fn f(value: f64) -> f32 {
    value as f32
}

fn show(app: &AppWindow, view: &View) {
    let (mut rects, mut circles, mut lines, mut labels) = (vec![], vec![], vec![], vec![]);
    for shape in &view.drawing.shapes {
        match shape {
            Shape::Rect {
                kind,
                x,
                y,
                width,
                height,
            } => rects.push(RectShape {
                x: f(*x),
                y: f(*y),
                w: f(*width),
                h: f(*height),
                kind: match kind {
                    ShapeKind::ToeKick => 0,
                    ShapeKind::Door => 2,
                    _ => 1,
                },
            }),
            Shape::Circle { cx, cy, radius, .. } => circles.push(CircleShape {
                cx: f(*cx),
                cy: f(*cy),
                r: f(*radius),
            }),
            Shape::Line { x1, y1, x2, y2 } => lines.push(LineShape {
                x1: f(*x1),
                y1: f(*y1),
                x2: f(*x2),
                y2: f(*y2),
            }),
            Shape::Label {
                x,
                y,
                text,
                vertical,
            } => labels.push(LabelShape {
                x: f(*x),
                y: f(*y),
                text: text.into(),
                vertical: *vertical,
            }),
        }
    }
    let d = &view.drawing;
    app.set_min_x(f(d.min_x));
    app.set_min_y(f(d.min_y));
    app.set_max_x(f(d.max_x));
    app.set_max_y(f(d.max_y));
    app.set_rects(ModelRc::new(VecModel::from(rects)));
    app.set_circles(ModelRc::new(VecModel::from(circles)));
    app.set_lines(ModelRc::new(VecModel::from(lines)));
    app.set_labels(ModelRc::new(VecModel::from(labels)));

    let rows: Vec<ModelRc<StandardListViewItem>> = view
        .cut_list
        .iter()
        .map(|r| {
            let cells: Vec<StandardListViewItem> = [
                r.part.as_str(),
                &r.material,
                &r.qty.to_string(),
                &r.length,
                &r.width,
                &r.thickness,
            ]
            .into_iter()
            .map(|s| StandardListViewItem::from(SharedString::from(s)))
            .collect();
            ModelRc::new(VecModel::from(cells))
        })
        .collect();
    app.set_rows(ModelRc::new(VecModel::from(rows)));
    app.set_error(SharedString::new());
}

fn read_params(app: &AppWindow) -> Result<Params, String> {
    let parse = |label: &str, text: SharedString| {
        Length::parse_inches(&text).ok_or_else(|| format!("{label}: cannot read \"{text}\""))
    };
    Ok(Params {
        width: parse("Width", app.get_width_text())?,
        height: parse("Height", app.get_height_text())?,
        depth: parse("Depth", app.get_depth_text())?,
        doors: u32::try_from(app.get_doors()).map_err(|e| e.to_string())?,
    })
}

/// Creates the window, fills it from a new session and wires the editing
/// callback.
///
/// # Errors
/// When the platform cannot create a window.
///
/// # Panics
/// If the embedded fixture does not evaluate, which is a build error.
pub fn create() -> Result<AppWindow, slint::PlatformError> {
    let app = AppWindow::new()?;
    let session = Rc::new(RefCell::new(Session::new()));

    let params = session.borrow().params();
    let inches = |l: Length| {
        SharedString::from(l.to_fractional_inches(32).to_string().trim_end_matches('"'))
    };
    app.set_width_text(inches(params.width));
    app.set_height_text(inches(params.height));
    app.set_depth_text(inches(params.depth));
    app.set_doors(i32::try_from(params.doors).unwrap_or(1));
    show(&app, &session.borrow().view().expect("fixture evaluates"));

    let weak = app.as_weak();
    app.on_edited(move || {
        let Some(app) = weak.upgrade() else { return };
        let result = read_params(&app)
            .and_then(|p| session.borrow_mut().apply(p).map_err(|e| e.to_string()));
        match result {
            Ok(view) => show(&app, &view),
            Err(message) => app.set_error(message.into()),
        }
    });
    Ok(app)
}
