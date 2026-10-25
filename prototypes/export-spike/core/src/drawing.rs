// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Front elevation of a product as resolution-independent shapes.
//!
//! Coordinates are millimeters with the origin at the product's bottom-left
//! corner on the floor and +y up. Front ends only map them to the screen.

use serde::Serialize;

use crate::eval::Evaluation;
use crate::length::Length;
use crate::model::{Product, Standard};

const DIMENSION_OFFSET_MM: f64 = 40.0;
const EXTENSION_OVERSHOOT_MM: f64 = 10.0;
const LABEL_GAP_MM: f64 = 14.0;
const MARGIN_MM: f64 = 90.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShapeKind {
    ToeKick,
    Case,
    Door,
    HingeCup,
    Dimension,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Shape {
    Rect {
        kind: ShapeKind,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Circle {
        kind: ShapeKind,
        cx: f64,
        cy: f64,
        radius: f64,
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Label {
        x: f64,
        y: f64,
        text: String,
        vertical: bool,
    },
}

/// A drawing and the bounding box that contains all its shapes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drawing {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub shapes: Vec<Shape>,
}

/// Builds the front elevation of an evaluated product.
pub fn elevation(product: &Product, standard: &Standard, evaluation: &Evaluation<'_>) -> Drawing {
    let mm = Length::to_mm_f64;
    let (w, h) = (mm(product.width), mm(product.height));
    let kick = mm(standard.toe_kick.height);
    let gap = mm(standard.fronts.gap);

    let mut shapes = vec![
        Shape::Rect {
            kind: ShapeKind::ToeKick,
            x: 0.0,
            y: 0.0,
            width: w,
            height: kick,
        },
        Shape::Rect {
            kind: ShapeKind::Case,
            x: 0.0,
            y: kick,
            width: w,
            height: h - kick,
        },
    ];

    // Doors overlay the box, separated and surrounded by the front gap.
    let door_y = kick + gap / 2.0;
    let mut door_x = gap / 2.0;
    for door in evaluation
        .parts
        .iter()
        .filter(|p| p.key.starts_with("front/door."))
    {
        let (door_w, door_h) = (mm(door.width), mm(door.length));
        shapes.push(Shape::Rect {
            kind: ShapeKind::Door,
            x: door_x,
            y: door_y,
            width: door_w,
            height: door_h,
        });

        // Hinge holes are in part coordinates: x along the length from the
        // bottom end, y across the width from the left edge.
        for hole in evaluation.holes.iter().filter(|h| h.part == door.key) {
            shapes.push(Shape::Circle {
                kind: ShapeKind::HingeCup,
                cx: door_x + mm(hole.y),
                cy: door_y + mm(hole.x),
                radius: mm(hole.diameter) / 2.0,
            });
        }
        door_x += door_w + gap;
    }

    let label = |length: Length| length.to_fractional_inches(32).to_string();
    let d = DIMENSION_OFFSET_MM;
    shapes.extend(horizontal_dimension(0.0, w, 0.0, -d, label(product.width)));
    shapes.extend(vertical_dimension(0.0, h, w, w + d, label(product.height)));
    shapes.extend(vertical_dimension(
        0.0,
        kick,
        0.0,
        -d,
        label(standard.toe_kick.height),
    ));

    Drawing {
        min_x: -MARGIN_MM,
        min_y: -MARGIN_MM,
        max_x: w + MARGIN_MM,
        max_y: h + MARGIN_MM / 4.0,
        shapes,
    }
}

/// A dimension measured along x between `x1` and `x2`, drawn at height `at`
/// from features at height `from`.
fn horizontal_dimension(x1: f64, x2: f64, from: f64, at: f64, text: String) -> Vec<Shape> {
    let overshoot = EXTENSION_OVERSHOOT_MM * (at - from).signum();
    vec![
        Shape::Line {
            x1,
            y1: from,
            x2: x1,
            y2: at + overshoot,
        },
        Shape::Line {
            x1: x2,
            y1: from,
            x2,
            y2: at + overshoot,
        },
        Shape::Line {
            x1,
            y1: at,
            x2,
            y2: at,
        },
        // Labels are anchored at their center, just outside the dimension line.
        Shape::Label {
            x: f64::midpoint(x1, x2),
            y: at + LABEL_GAP_MM * (at - from).signum(),
            text,
            vertical: false,
        },
    ]
}

/// A dimension measured along y between `y1` and `y2`, drawn at `at` from
/// features at `from`.
fn vertical_dimension(y1: f64, y2: f64, from: f64, at: f64, text: String) -> Vec<Shape> {
    let overshoot = EXTENSION_OVERSHOOT_MM * (at - from).signum();
    vec![
        Shape::Line {
            x1: from,
            y1,
            x2: at + overshoot,
            y2: y1,
        },
        Shape::Line {
            x1: from,
            y1: y2,
            x2: at + overshoot,
            y2,
        },
        Shape::Line {
            x1: at,
            y1,
            x2: at,
            y2,
        },
        Shape::Label {
            x: at + LABEL_GAP_MM * (at - from).signum(),
            y: f64::midpoint(y1, y2),
            text,
            vertical: true,
        },
    ]
}
