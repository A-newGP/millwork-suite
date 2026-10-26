// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The drawing area's content: the elevation of a wall, or one cabinet
//! alone, as path commands the window scales to fit and text labels.
//!
//! Coordinates are millimeters from the drawing's top left corner, `y`
//! down, as the window's paths expect; elevations have `y` up, so they
//! are flipped here.

use std::fmt::Write as _;

use opencabinet_eval::front::Rect;
use opencabinet_eval::{Dimension, Elevation, FrontKind, FrontView};
use opencabinet_model::{Id, Length, Side};

/// What the drawing area shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    /// What it is, for screen readers: "Elevation of wall North".
    pub title: String,
    /// Its size in millimeters.
    pub width: f32,
    /// Its height in millimeters.
    pub height: f32,
    /// Walls, openings, cabinets and fronts.
    pub lines: String,
    /// Hinge marks on doors, drawn lighter.
    pub marks: String,
    /// The outline of the cabinet being edited.
    pub highlight: String,
    /// Dimension lines.
    pub dimensions: String,
    /// Labels and dimension values.
    pub labels: Vec<Label>,
}

/// A text centered on a point.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    /// From the drawing's left, in millimeters.
    pub x: f32,
    /// From the drawing's top, in millimeters.
    pub y: f32,
    /// What it says.
    pub text: String,
}

/// Room around a drawing for its dimensions, in millimeters.
const MARGIN: Length = Length::from_mm(300);
/// The length of a dimension's end ticks, in millimeters.
const TICK: f64 = 25.0;

#[allow(clippy::cast_precision_loss, reason = "a room is far below 2^53 nm")]
fn mm(length: Length) -> f64 {
    length.nm() as f64 / 1_000_000.0
}

#[allow(clippy::cast_possible_truncation, reason = "screen coordinates")]
fn f32_of(value: f64) -> f32 {
    value as f32
}

/// Paths for one drawing, which maps a rectangle of the elevation, from
/// `left` and up to `top`, to the drawing's top left corner.
struct Pen {
    left: f64,
    top: f64,
}

impl Pen {
    fn point(&self, x: f64, y: f64) -> (f64, f64) {
        (x - self.left, self.top - y)
    }

    fn line(&self, path: &mut String, from: (f64, f64), to: (f64, f64)) {
        let (a, b) = (self.point(from.0, from.1), self.point(to.0, to.1));
        write!(path, "M {:.2} {:.2} L {:.2} {:.2} ", a.0, a.1, b.0, b.1).expect("to a string");
    }

    fn rect(&self, path: &mut String, rect: Rect, at: (f64, f64)) {
        let (left, bottom) = (at.0 + mm(rect.x), at.1 + mm(rect.y));
        let (right, top) = (left + mm(rect.width), bottom + mm(rect.height));
        let (a, b) = (self.point(left, top), self.point(right, bottom));
        write!(
            path,
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            a.0, a.1, b.0, a.1, b.0, b.1, a.0, b.1
        )
        .expect("to a string");
    }

    fn label(&self, x: f64, y: f64, text: impl Into<String>) -> Label {
        let (x, y) = self.point(x, y);
        Label {
            x: f32_of(x),
            y: f32_of(y),
            text: text.into(),
        }
    }
}

/// Draws a front view at `at`: the outline, toe kick, face frame and
/// fronts, with hinge marks: lines from the corners of the handle side to
/// the middle of the hinge side.
fn front(pen: &Pen, drawing: &mut Drawing, view: &FrontView, at: (f64, f64)) {
    for rect in view.toe_kick.iter().chain(&view.frame) {
        pen.rect(&mut drawing.lines, *rect, at);
    }
    for shape in &view.fronts {
        pen.rect(&mut drawing.lines, shape.rect, at);
        let FrontKind::Door { hinge: Some(side) } = shape.kind else {
            continue;
        };
        let r = shape.rect;
        let (x0, y0) = (at.0 + mm(r.x), at.1 + mm(r.y));
        let (x1, y1) = (x0 + mm(r.width), y0 + mm(r.height));
        let (hinge, handle) = match side {
            Side::Left => (x0, x1),
            Side::Right => (x1, x0),
        };
        let middle = (hinge, f64::midpoint(y0, y1));
        pen.line(&mut drawing.marks, (handle, y0), middle);
        pen.line(&mut drawing.marks, (handle, y1), middle);
    }
}

/// The elevation of a wall, with the placements of `selected` outlined;
/// `show` writes a length in the user's units.
#[must_use]
pub fn wall(elevation: &Elevation, selected: Id, show: &dyn Fn(Length) -> String) -> Drawing {
    let (length, height) = (mm(elevation.length), mm(elevation.height));
    let margin = mm(MARGIN);
    let pen = Pen {
        left: -margin,
        top: height + margin,
    };
    let mut drawing = Drawing {
        title: format!("Elevation of wall {}", elevation.name),
        width: f32_of(length + 2.0 * margin),
        height: f32_of(height + 2.0 * margin),
        ..Drawing::default()
    };
    let whole = Rect {
        x: Length::ZERO,
        y: Length::ZERO,
        width: elevation.length,
        height: elevation.height,
    };
    pen.rect(&mut drawing.lines, whole, (0.0, 0.0));
    for opening in &elevation.openings {
        pen.rect(&mut drawing.lines, opening.rect, (0.0, 0.0));
    }
    for cabinet in &elevation.cabinets {
        let at = (mm(cabinet.rect.x), mm(cabinet.rect.y));
        let outline = Rect {
            x: Length::ZERO,
            y: Length::ZERO,
            ..cabinet.rect
        };
        pen.rect(&mut drawing.lines, outline, at);
        if let Some(view) = &cabinet.view {
            front(&pen, &mut drawing, view, at);
        }
        if cabinet.product == selected {
            pen.rect(&mut drawing.highlight, outline, at);
        }
        let center = (
            at.0 + mm(cabinet.rect.width) / 2.0,
            at.1 + mm(cabinet.rect.height) / 2.0,
        );
        drawing
            .labels
            .push(pen.label(center.0, center.1, cabinet.label.clone()));
    }
    for dimension in elevation.dimensions() {
        let (from, to, label) = match dimension {
            Dimension::Horizontal { from, to, at } => {
                let y = mm(at);
                let (a, b) = ((mm(from), y), (mm(to), y));
                (
                    a,
                    b,
                    pen.label(f64::midpoint(a.0, b.0), y + TICK * 1.5, show(to - from)),
                )
            }
            Dimension::Vertical { from, to, at } => {
                let x = mm(at);
                let (a, b) = ((x, mm(from)), (x, mm(to)));
                (
                    a,
                    b,
                    pen.label(x + TICK * 4.0, f64::midpoint(a.1, b.1), show(to - from)),
                )
            }
        };
        pen.line(&mut drawing.dimensions, from, to);
        for end in [from, to] {
            let tick = (end.0 - TICK / 2.0, end.1 - TICK / 2.0);
            pen.line(
                &mut drawing.dimensions,
                tick,
                (tick.0 + TICK, tick.1 + TICK),
            );
        }
        drawing.labels.push(label);
    }
    drawing
}

/// One cabinet that is not placed on a wall, seen from the front.
#[must_use]
pub fn cabinet(label: &str, width: Length, height: Length, view: Option<&FrontView>) -> Drawing {
    let margin = mm(MARGIN) / 2.0;
    let (w, h) = (mm(width), mm(height));
    let pen = Pen {
        left: -margin,
        top: h + margin,
    };
    let mut drawing = Drawing {
        title: format!("{label}, not placed on a wall"),
        width: f32_of(w + 2.0 * margin),
        height: f32_of(h + 2.0 * margin),
        ..Drawing::default()
    };
    let outline = Rect {
        x: Length::ZERO,
        y: Length::ZERO,
        width,
        height,
    };
    pen.rect(&mut drawing.lines, outline, (0.0, 0.0));
    pen.rect(&mut drawing.highlight, outline, (0.0, 0.0));
    if let Some(view) = view {
        front(&pen, &mut drawing, view, (0.0, 0.0));
    }
    drawing.labels.push(pen.label(w / 2.0, h / 2.0, label));
    drawing
}
