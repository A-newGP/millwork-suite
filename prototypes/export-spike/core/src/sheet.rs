// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sheets as a short list of marks (lines, circles, text) on named layers,
//! in millimeters with +y up. Every exporter draws these same marks, so each
//! one only has to map three primitives to its file format.

use crate::drawing::{Shape, ShapeKind};
use crate::{CutRow, Job};

const SHEET_GAP_MM: f64 = 200.0;
const LABEL_HEIGHT_MM: f64 = 25.0;
const DIMENSION_TEXT_MM: f64 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Case,
    Fronts,
    Machining,
    Dimensions,
    Text,
}

impl Layer {
    pub const ALL: [Self; 5] = [
        Self::Case,
        Self::Fronts,
        Self::Machining,
        Self::Dimensions,
        Self::Text,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Case => "CASE",
            Self::Fronts => "FRONTS",
            Self::Machining => "MACHINING",
            Self::Dimensions => "DIMENSIONS",
            Self::Text => "TEXT",
        }
    }

    /// AutoCAD Color Index, the palette every DXF reader understands.
    pub fn color_index(self) -> u8 {
        match self {
            Self::Case => 7,
            Self::Fronts => 5,
            Self::Machining => 1,
            Self::Dimensions => 3,
            Self::Text => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    /// Centered on the anchor, horizontally and vertically.
    Center,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    Line {
        layer: Layer,
        from: (f64, f64),
        to: (f64, f64),
    },
    Circle {
        layer: Layer,
        center: (f64, f64),
        radius: f64,
    },
    Text {
        layer: Layer,
        at: (f64, f64),
        height: f64,
        text: String,
        align: Align,
        vertical: bool,
    },
}

/// A set of marks and the box that contains them.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub title: String,
    pub min: (f64, f64),
    pub max: (f64, f64),
    pub marks: Vec<Mark>,
}

/// One sheet per product elevation.
pub fn elevations(job: &Job) -> Vec<Sheet> {
    job.elevations
        .iter()
        .map(|e| {
            let d = &e.drawing;
            let mut marks = Vec::new();
            for shape in &d.shapes {
                shape_marks(shape, &mut marks);
            }
            Sheet {
                title: format!("{} — {}", job.name, e.label),
                min: (d.min_x, d.min_y),
                max: (d.max_x, d.max_y),
                marks,
            }
        })
        .collect()
}

/// Every elevation side by side on one sheet, each labeled below, for CAD.
pub fn elevations_side_by_side(job: &Job) -> Sheet {
    let mut marks = Vec::new();
    let mut x = 0.0;
    let (mut min_y, mut max_y): (f64, f64) = (0.0, 0.0);
    for sheet in elevations(job) {
        let dx = x - sheet.min.0;
        marks.extend(sheet.marks.into_iter().map(|m| translate(m, dx)));
        let label = sheet
            .title
            .rsplit(" — ")
            .next()
            .unwrap_or_default()
            .to_owned();
        marks.push(Mark::Text {
            layer: Layer::Text,
            at: (x - sheet.min.0, sheet.min.1 - LABEL_HEIGHT_MM),
            height: LABEL_HEIGHT_MM,
            text: label,
            align: Align::Left,
            vertical: false,
        });
        x += sheet.max.0 - sheet.min.0 + SHEET_GAP_MM;
        min_y = min_y.min(sheet.min.1 - LABEL_HEIGHT_MM);
        max_y = max_y.max(sheet.max.1);
    }
    Sheet {
        title: format!("{} — elevations", job.name),
        min: (0.0, min_y),
        max: (x - SHEET_GAP_MM, max_y),
        marks,
    }
}

/// Every machined part outline with its holes, side by side, for CNC.
pub fn machining(job: &Job) -> Sheet {
    let mut marks = Vec::new();
    let mut x = 0.0;
    let mut max_y: f64 = 0.0;
    for part in &job.machined {
        // Part x (length) runs up the sheet, so doors stand upright.
        let (w, h) = (part.width, part.length);
        rect(&mut marks, Layer::Case, x, 0.0, w, h);
        for hole in &part.holes {
            marks.push(Mark::Circle {
                layer: Layer::Machining,
                center: (x + hole.y, hole.x),
                radius: hole.diameter / 2.0,
            });
        }
        marks.push(Mark::Text {
            layer: Layer::Text,
            at: (x, -LABEL_HEIGHT_MM * 1.5),
            height: DIMENSION_TEXT_MM,
            text: part.name.clone(),
            align: Align::Left,
            vertical: false,
        });
        x += w + SHEET_GAP_MM;
        max_y = max_y.max(h);
    }
    Sheet {
        title: format!("{} — machining", job.name),
        min: (0.0, -LABEL_HEIGHT_MM * 2.0),
        max: (x - SHEET_GAP_MM, max_y),
        marks,
    }
}

/// The cut list laid out as a table on pages of `size` millimeters, with a
/// `margin` all around and room for the sheet title at the top. Columns are proportional to their usual content.
pub fn cut_list_pages(job: &Job, size: (f64, f64), margin: f64) -> Vec<Sheet> {
    const WIDTHS: [f64; 7] = [0.08, 0.27, 0.27, 0.06, 0.11, 0.11, 0.10];
    const ROW_MM: f64 = 6.0;
    const TEXT_MM: f64 = 3.0;
    let table_width = size.0 - 2.0 * margin;
    let top = size.1 - margin - 10.0;
    let rows_per_page = ((top - margin) / ROW_MM) as usize - 1;

    let chunks: Vec<&[CutRow]> = job.cut_list.chunks(rows_per_page.max(1)).collect();
    let pages = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(page, rows)| {
            let mut marks = Vec::new();
            let cells = std::iter::once(CutRow::HEADERS.map(String::from))
                .chain(rows.iter().map(CutRow::cells));
            for (i, row) in cells.enumerate() {
                let y = top - ROW_MM * i as f64;
                let mut x = margin;
                for (cell, share) in row.into_iter().zip(WIDTHS) {
                    marks.push(Mark::Text {
                        layer: Layer::Text,
                        at: (x + 1.0, y - ROW_MM + 1.8),
                        height: TEXT_MM,
                        text: cell,
                        align: Align::Left,
                        vertical: false,
                    });
                    x += share * table_width;
                }
                marks.push(Mark::Line {
                    layer: Layer::Dimensions,
                    from: (margin, y - ROW_MM),
                    to: (margin + table_width, y - ROW_MM),
                });
            }
            Sheet {
                title: format!("{} — cut list, page {} of {pages}", job.name, page + 1),
                min: (0.0, 0.0),
                max: size,
                marks,
            }
        })
        .collect()
}

fn shape_marks(shape: &Shape, marks: &mut Vec<Mark>) {
    match shape {
        Shape::Rect {
            kind,
            x,
            y,
            width,
            height,
        } => rect(marks, layer_of(*kind), *x, *y, *width, *height),
        Shape::Circle {
            kind,
            cx,
            cy,
            radius,
        } => marks.push(Mark::Circle {
            layer: layer_of(*kind),
            center: (*cx, *cy),
            radius: *radius,
        }),
        Shape::Line { x1, y1, x2, y2 } => marks.push(Mark::Line {
            layer: Layer::Dimensions,
            from: (*x1, *y1),
            to: (*x2, *y2),
        }),
        Shape::Label {
            x,
            y,
            text,
            vertical,
        } => marks.push(Mark::Text {
            layer: Layer::Dimensions,
            at: (*x, *y),
            height: DIMENSION_TEXT_MM,
            text: text.clone(),
            align: Align::Center,
            vertical: *vertical,
        }),
    }
}

fn layer_of(kind: ShapeKind) -> Layer {
    match kind {
        ShapeKind::ToeKick | ShapeKind::Case => Layer::Case,
        ShapeKind::Door => Layer::Fronts,
        ShapeKind::HingeCup => Layer::Machining,
        ShapeKind::Dimension => Layer::Dimensions,
    }
}

fn rect(marks: &mut Vec<Mark>, layer: Layer, x: f64, y: f64, w: f64, h: f64) {
    let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for i in 0..4 {
        marks.push(Mark::Line {
            layer,
            from: corners[i],
            to: corners[(i + 1) % 4],
        });
    }
}

fn translate(mark: Mark, dx: f64) -> Mark {
    let t = |(x, y): (f64, f64)| (x + dx, y);
    match mark {
        Mark::Line { layer, from, to } => Mark::Line {
            layer,
            from: t(from),
            to: t(to),
        },
        Mark::Circle {
            layer,
            center,
            radius,
        } => Mark::Circle {
            layer,
            center: t(center),
            radius,
        },
        Mark::Text {
            layer,
            at,
            height,
            text,
            align,
            vertical,
        } => Mark::Text {
            layer,
            at: t(at),
            height,
            text,
            align,
            vertical,
        },
    }
}

/// US Letter, landscape, in millimeters: the usual shop drawing sheet.
pub const LETTER_LANDSCAPE: (f64, f64) = (279.4, 215.9);
pub const PAGE_MARGIN_MM: f64 = 12.7;

/// Maps sheet coordinates onto a page: `page = sheet * scale + offset`, both
/// in millimeters with +y up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    pub scale: f64,
    pub offset: (f64, f64),
}

impl Fit {
    /// The largest scale that fits `sheet` inside the page margins, below a
    /// band kept for the title, centered.
    pub fn new(sheet: &Sheet, page: (f64, f64), margin: f64) -> Self {
        let title_band = 12.0;
        let (w, h) = (sheet.max.0 - sheet.min.0, sheet.max.1 - sheet.min.1);
        let (room_w, room_h) = (page.0 - 2.0 * margin, page.1 - 2.0 * margin - title_band);
        let scale = (room_w / w).min(room_h / h);
        let offset = (
            margin + (room_w - w * scale) / 2.0 - sheet.min.0 * scale,
            margin + (room_h - h * scale) / 2.0 - sheet.min.1 * scale,
        );
        Self { scale, offset }
    }

    pub const IDENTITY: Self = Self {
        scale: 1.0,
        offset: (0.0, 0.0),
    };

    pub fn point(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            x * self.scale + self.offset.0,
            y * self.scale + self.offset.1,
        )
    }
}

/// An estimate of a text's width, for centering labels. Neither PDF library
/// measures text without a shaping library, so this uses the average
/// advance of sans-serif figures, about 0.55 em.
pub fn approx_text_width(text: &str, height: f64) -> f64 {
    0.55 * height * text.chars().count() as f64
}
