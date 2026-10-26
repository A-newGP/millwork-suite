// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Parts as DXF (ADR-0015): one file per part for CAM programs, and one
//! with every part laid out by material for people.
//!
//! Each part is drawn at its cut size, the size the machine cuts before
//! the edge band goes on, from its corner at the origin: `X` along its
//! length, which is along the grain, and `Y` across it, seen from face A.
//! Holes and grooves are shifted by the bands on the `x0` and `y0` edges,
//! so they land where the evaluator put them once the part is banded.
//!
//! Layers name the tool and depth, so an operator maps each layer to a
//! tool once in the CAM program: `OUTLINE` for the part's contour,
//! `HOLE_A_D5_Z12` for 5 mm holes 12 mm deep in face A,
//! `GROOVE_A_W12_Z9.525` for 12 mm grooves 9.525 mm deep, `LABEL` for the
//! texts. Files are DXF 2000, the oldest version that declares its units,
//! in millimeters.

use std::collections::BTreeSet;

use chrono::{Local, NaiveDate, TimeZone, Utc};
use dxf::entities::{Circle, Entity, EntityType, Line, LwPolyline, Text};
use dxf::enums::{AcadVersion, Units};
use dxf::tables::Layer;
use dxf::{Color, Drawing, LwPolylineVertex, Point};
use opencabinet_eval::{Face, Operation, Part, evaluate};
use opencabinet_model::{Grain, Length, Library, Project};

use crate::cut_list::part_name;

/// A DXF file, named for the part it draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxfFile {
    /// A file name, such as `B30 - Left side.dxf`.
    pub name: String,
    /// The file's bytes.
    pub contents: Vec<u8>,
}

/// The name of the file with every part.
pub const ALL_PARTS: &str = "all-parts.dxf";

/// One DXF per part of every product that could be evaluated, in the
/// cut list's order, then [`ALL_PARTS`] with all of them.
///
/// # Errors
/// When a drawing cannot be written.
pub fn parts_dxf(project: &Project, library: &Library) -> Result<Vec<DxfFile>, String> {
    let evaluation = evaluate(project, library);
    let sketches: Vec<(String, Sketch)> = evaluation
        .parts
        .iter()
        .map(|part| sketch(part, project, library))
        .collect();

    let mut files = Vec::with_capacity(sketches.len() + 1);
    let mut names = BTreeSet::new();
    for (name, sketch) in &sketches {
        // Two parts with one name, as when two products share a label, get
        // numbered rather than overwritten.
        let mut file = format!("{}.dxf", safe_file_name(name));
        let mut n = 2;
        while !names.insert(file.clone()) {
            file = format!("{} ({n}).dxf", safe_file_name(name));
            n += 1;
        }
        files.push(DxfFile {
            name: file,
            contents: write(&sketch.shapes)?,
        });
    }
    let layout = lay_out(&sketches, &evaluation.parts, library);
    files.push(DxfFile {
        name: ALL_PARTS.into(),
        contents: write(&layout)?,
    });
    Ok(files)
}

/// Something drawn, in millimeters.
#[derive(Clone, Debug, PartialEq)]
enum Shape {
    /// A closed polyline.
    Polygon {
        layer: String,
        points: Vec<(f64, f64)>,
    },
    Circle {
        layer: String,
        center: (f64, f64),
        radius: f64,
    },
    Line {
        layer: String,
        from: (f64, f64),
        to: (f64, f64),
    },
    /// Left-aligned text, `at` its baseline's start.
    Text {
        layer: String,
        at: (f64, f64),
        height: f64,
        text: String,
    },
}

impl Shape {
    fn moved(&self, (dx, dy): (f64, f64)) -> Self {
        let shift = |(x, y): (f64, f64)| (x + dx, y + dy);
        match self {
            Self::Polygon { layer, points } => Self::Polygon {
                layer: layer.clone(),
                points: points.iter().copied().map(shift).collect(),
            },
            Self::Circle {
                layer,
                center,
                radius,
            } => Self::Circle {
                layer: layer.clone(),
                center: shift(*center),
                radius: *radius,
            },
            Self::Line { layer, from, to } => Self::Line {
                layer: layer.clone(),
                from: shift(*from),
                to: shift(*to),
            },
            Self::Text {
                layer,
                at,
                height,
                text,
            } => Self::Text {
                layer: layer.clone(),
                at: shift(*at),
                height: *height,
                text: text.clone(),
            },
        }
    }

    fn layer(&self) -> &str {
        match self {
            Self::Polygon { layer, .. }
            | Self::Circle { layer, .. }
            | Self::Line { layer, .. }
            | Self::Text { layer, .. } => layer,
        }
    }
}

/// A part's drawing and the box it takes, label included.
struct Sketch {
    shapes: Vec<Shape>,
    /// Lowest and highest corners.
    bounds: ((f64, f64), (f64, f64)),
}

const OUTLINE: &str = "OUTLINE";
const LABEL: &str = "LABEL";
const TEXT_HEIGHT: f64 = 10.0;
const LINE_SPACING: f64 = 15.0;

/// Millimeters as a DXF coordinate.
#[allow(clippy::cast_precision_loss, reason = "a part is far below 2^53 nm")]
fn mm(length: Length) -> f64 {
    length.nm() as f64 / 1_000_000.0
}

/// Millimeters as text, with only the decimals they need.
fn mm_text(length: Length) -> String {
    let nm = length.nm();
    let sign = if nm < 0 { "-" } else { "" };
    let text = format!(
        "{sign}{}.{:06}",
        (nm / 1_000_000).abs(),
        (nm % 1_000_000).abs()
    );
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn face_letter(face: Face) -> &'static str {
    match face {
        Face::A => "A",
        Face::B => "B",
    }
}

/// The name and drawing of one part.
fn sketch(part: &Part, project: &Project, library: &Library) -> (String, Sketch) {
    let band = |edge: usize| {
        part.edges[edge]
            .and_then(|id| library.edge_bands.iter().find(|b| b.id == id))
            .map_or(Length::ZERO, |b| b.thickness)
    };
    // Operations are placed on the banded part; the cut part starts the
    // bands' thickness further in.
    let (dx, dy) = (band(0), band(2));
    let (length, width) = (part.cut_length, part.cut_width);

    let mut shapes = vec![Shape::Polygon {
        layer: OUTLINE.into(),
        points: vec![
            (0.0, 0.0),
            (mm(length), 0.0),
            (mm(length), mm(width)),
            (0.0, mm(width)),
        ],
    }];
    shapes.extend(operation_shapes(part, (dx, dy)));

    let product = project
        .products
        .iter()
        .find(|p| p.id == part.product)
        .map_or("", |p| p.label.as_str());
    let name = format!("{product} - {}", part_name(part));
    let material = library.materials.iter().find(|m| m.id == part.material);
    let banded: Vec<&str> = ["left", "right", "bottom", "top"]
        .into_iter()
        .zip(part.edges)
        .filter_map(|(edge, band)| band.map(|_| edge))
        .collect();
    let lines = [
        name.clone(),
        format!(
            "{}, {} mm, qty {}",
            material.map_or("", |m| m.name.as_str()),
            mm_text(part.thickness),
            part.quantity
        ),
        if banded.is_empty() {
            format!("Cut {} x {} mm", mm_text(length), mm_text(width))
        } else {
            format!(
                "Cut {} x {} mm, edge band: {}",
                mm_text(length),
                mm_text(width),
                banded.join(", ")
            )
        },
    ];
    // The label sits above the part, its first line on top.
    let base = mm(width) + LINE_SPACING;
    let mut widest = mm(length);
    for (i, line) in lines.iter().rev().enumerate() {
        #[allow(clippy::cast_precision_loss, reason = "a short line of text")]
        let estimate = line.chars().count() as f64 * TEXT_HEIGHT * 0.7;
        widest = widest.max(estimate);
        #[allow(clippy::cast_precision_loss, reason = "three lines")]
        let y = base + i as f64 * LINE_SPACING;
        shapes.push(Shape::Text {
            layer: LABEL.into(),
            at: (0.0, y),
            height: TEXT_HEIGHT,
            text: line.clone(),
        });
    }
    let top = base + 2.0 * LINE_SPACING + TEXT_HEIGHT;
    // An arrow under the part shows the grain, along X.
    let mut bottom = 0.0;
    if material.is_some_and(|m| m.grain == Grain::Length) {
        let y = -LINE_SPACING;
        let end = mm(length).min(100.0);
        let tip = (end, y);
        shapes.extend([
            Shape::Line {
                layer: LABEL.into(),
                from: (0.0, y),
                to: tip,
            },
            Shape::Line {
                layer: LABEL.into(),
                from: (end - 5.0, y + 3.0),
                to: tip,
            },
            Shape::Line {
                layer: LABEL.into(),
                from: (end - 5.0, y - 3.0),
                to: tip,
            },
            Shape::Text {
                layer: LABEL.into(),
                at: (end + 5.0, y - TEXT_HEIGHT / 2.0),
                height: TEXT_HEIGHT,
                text: "grain".into(),
            },
        ]);
        bottom = y - TEXT_HEIGHT;
    }
    let bounds = ((0.0, bottom), (widest, top));
    (name, Sketch { shapes, bounds })
}

/// The holes and grooves of `part`, at its cut size: `shift` is the
/// thickness of the bands on its `x0` and `y0` edges.
fn operation_shapes(part: &Part, (dx, dy): (Length, Length)) -> Vec<Shape> {
    let (length, width) = (part.cut_length, part.cut_width);
    let at = |x: Length, y: Length| (mm(x - dx), mm(y - dy));
    let mut shapes = Vec::new();
    for operation in &part.operations {
        match operation {
            Operation::Hole {
                face,
                at: p,
                diameter,
                depth,
            } => shapes.push(hole(*face, at(p.x, p.y), *diameter, *depth)),
            Operation::HolePattern {
                face,
                start,
                step,
                count,
                diameter,
                depth,
            } => {
                let (mut x, mut y) = (start.x, start.y);
                for _ in 0..*count {
                    shapes.push(hole(*face, at(x, y), *diameter, *depth));
                    x += step.x;
                    y += step.y;
                }
            }
            Operation::Groove {
                face,
                start,
                end,
                width: groove_width,
                depth,
            } => {
                let layer = format!(
                    "GROOVE_{}_W{}_Z{}",
                    face_letter(*face),
                    mm_text(*groove_width),
                    mm_text(*depth)
                );
                let (from, to) = (at(start.x, start.y), at(end.x, end.y));
                let points = groove_outline(from, to, mm(*groove_width), (mm(length), mm(width)));
                shapes.push(Shape::Polygon { layer, points });
            }
        }
    }
    shapes
}

/// A hole on its tool's layer.
fn hole(face: Face, center: (f64, f64), diameter: Length, depth: Length) -> Shape {
    Shape::Circle {
        layer: format!(
            "HOLE_{}_D{}_Z{}",
            face_letter(face),
            mm_text(diameter),
            mm_text(depth)
        ),
        center,
        radius: mm(diameter) / 2.0,
    }
}

/// The area a groove `width` wide takes along its center line from `from`
/// to `to`. A groove along or across the part stops at its edges, which
/// a groove that runs through ends on once the part is cut.
fn groove_outline(
    from: (f64, f64),
    to: (f64, f64),
    width: f64,
    (length, part_width): (f64, f64),
) -> Vec<(f64, f64)> {
    let half = width / 2.0;
    let clip = |(x, y): (f64, f64)| (x.clamp(0.0, length), y.clamp(0.0, part_width));
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    if dy == 0.0 || dx == 0.0 {
        let (x0, x1) = (
            from.0.min(to.0) - half * f64::from(u8::from(dx == 0.0)),
            from.0.max(to.0) + half * f64::from(u8::from(dx == 0.0)),
        );
        let (y0, y1) = (
            from.1.min(to.1) - half * f64::from(u8::from(dy == 0.0)),
            from.1.max(to.1) + half * f64::from(u8::from(dy == 0.0)),
        );
        return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
            .into_iter()
            .map(clip)
            .collect();
    }
    let norm = dx.hypot(dy);
    let (nx, ny) = (-dy / norm * half, dx / norm * half);
    vec![
        (from.0 + nx, from.1 + ny),
        (to.0 + nx, to.1 + ny),
        (to.0 - nx, to.1 - ny),
        (from.0 - nx, from.1 - ny),
    ]
}

/// Every part laid out for people: a row per material, in the order the
/// materials first appear, parts left to right in the cut list's order.
fn lay_out(sketches: &[(String, Sketch)], parts: &[Part], library: &Library) -> Vec<Shape> {
    const GAP: f64 = 100.0;
    let mut materials = Vec::new();
    for part in parts {
        if !materials.contains(&part.material) {
            materials.push(part.material);
        }
    }
    let mut shapes = Vec::new();
    let mut row_top = 0.0;
    for material in materials {
        let row: Vec<&Sketch> = sketches
            .iter()
            .zip(parts)
            .filter(|(_, part)| part.material == material)
            .map(|((_, sketch), _)| sketch)
            .collect();
        let height = row
            .iter()
            .map(|s| s.bounds.1.1 - s.bounds.0.1)
            .fold(0.0, f64::max);
        let name = library
            .materials
            .iter()
            .find(|m| m.id == material)
            .map_or("", |m| m.name.as_str());
        shapes.push(Shape::Text {
            layer: LABEL.into(),
            at: (0.0, row_top - 2.0 * TEXT_HEIGHT),
            height: 2.0 * TEXT_HEIGHT,
            text: name.into(),
        });
        let row_bottom = row_top - 3.0 * TEXT_HEIGHT - height;
        let mut left = 0.0;
        for sketch in row {
            let ((low_x, low_y), (high_x, _)) = sketch.bounds;
            let offset = (left - low_x, row_bottom - low_y);
            shapes.extend(sketch.shapes.iter().map(|s| s.moved(offset)));
            left += high_x - low_x + GAP;
        }
        row_top = row_bottom - GAP;
    }
    shapes
}

/// A file name without the characters Windows, macOS and Linux reject.
fn safe_file_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_owned()
}

/// The `AutoCAD` Color Index of a layer: white for outlines, red for holes,
/// blue for grooves, gray for labels.
fn color(layer: &str) -> u8 {
    if layer.starts_with("HOLE_") {
        1
    } else if layer.starts_with("GROOVE_") {
        5
    } else if layer == LABEL {
        8
    } else {
        7
    }
}

/// The shapes as a DXF 2000 file in millimeters.
fn write(shapes: &[Shape]) -> Result<Vec<u8>, String> {
    let mut drawing = Drawing::new();
    drawing.header.version = AcadVersion::R2000;
    drawing.header.default_drawing_units = Units::Millimeters;
    // The same parts always give the same bytes: the crate would stamp the
    // time and random identifiers. The dates are local wall-clock times in
    // the file, so a fixed one reads the same in every time zone.
    let epoch = NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .expect("a valid date");
    let local = Local
        .from_local_datetime(&epoch)
        .earliest()
        .ok_or("no local time for the fixed date")?;
    drawing.header.creation_date = local;
    drawing.header.update_date = local;
    drawing.header.creation_date_universal = Utc.from_utc_datetime(&epoch);
    drawing.header.update_date_universal = Utc.from_utc_datetime(&epoch);
    drawing.header.fingerprint_guid = uuid::Uuid::nil();
    drawing.header.version_guid = uuid::Uuid::nil();

    let mut layers: Vec<&str> = vec![OUTLINE];
    for shape in shapes {
        if !layers.contains(&shape.layer()) {
            layers.push(shape.layer());
        }
    }
    for name in layers {
        drawing.add_layer(Layer {
            name: name.into(),
            color: Color::from_index(color(name)),
            ..Default::default()
        });
    }

    let point = |(x, y): (f64, f64)| Point::new(x, y, 0.0);
    for shape in shapes {
        let specific = match shape {
            Shape::Polygon { points, .. } => {
                let mut polyline = LwPolyline {
                    vertices: points
                        .iter()
                        .map(|&(x, y)| LwPolylineVertex {
                            x,
                            y,
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                };
                polyline.set_is_closed(true);
                EntityType::LwPolyline(polyline)
            }
            Shape::Circle { center, radius, .. } => EntityType::Circle(Circle {
                center: point(*center),
                radius: *radius,
                ..Default::default()
            }),
            Shape::Line { from, to, .. } => EntityType::Line(Line {
                p1: point(*from),
                p2: point(*to),
                ..Default::default()
            }),
            Shape::Text {
                at, height, text, ..
            } => EntityType::Text(Text {
                location: point(*at),
                text_height: *height,
                value: text.clone(),
                ..Default::default()
            }),
        };
        let mut entity = Entity::new(specific);
        entity.common.layer = shape.layer().into();
        drawing.add_entity(entity);
    }
    drawing.normalize();

    let mut bytes = Vec::new();
    drawing
        .save(&mut bytes)
        .map_err(|e| format!("the DXF could not be written: {e}"))?;
    fix_table_headers(&bytes)
}

/// Gives each table header the handle and owner the `dxf` crate leaves
/// out, which readers otherwise have to repair (ADR-0015), and moves the
/// handle seed past the new handles.
fn fix_table_headers(bytes: &[u8]) -> Result<Vec<u8>, String> {
    const EOL: &[u8] = b"\r\n";
    fn code(line: &[u8]) -> Option<&str> {
        std::str::from_utf8(line).ok().map(str::trim)
    }
    let lines: Vec<&[u8]> = bytes
        .strip_suffix(EOL)
        .unwrap_or(bytes)
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect();
    if !lines.len().is_multiple_of(2) {
        return Err("the DXF has a group code without a value".into());
    }
    let pairs: Vec<(&[u8], &[u8])> = lines.chunks(2).map(|p| (p[0], p[1])).collect();
    let seed_at = pairs
        .iter()
        .position(|&(c, v)| code(c) == Some("9") && v == b"$HANDSEED")
        .ok_or("the DXF has no handle seed")?
        + 1;
    let mut seed = std::str::from_utf8(pairs[seed_at].1)
        .ok()
        .and_then(|v| u64::from_str_radix(v.trim(), 16).ok())
        .ok_or("the DXF's handle seed is not a handle")?;

    let mut out: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(pairs.len() + 32);
    let mut i = 0;
    while i < pairs.len() {
        let (c, v) = pairs[i];
        out.push((c.to_vec(), v.to_vec()));
        let opens_table = code(c) == Some("0") && v == b"TABLE";
        if opens_table && pairs.get(i + 1).is_some_and(|&(c, _)| code(c) == Some("2")) {
            let (name_code, name) = pairs[i + 1];
            out.push((name_code.to_vec(), name.to_vec()));
            out.push((b"  5".to_vec(), format!("{seed:X}").into_bytes()));
            out.push((b"330".to_vec(), b"0".to_vec()));
            seed += 1;
            i += 2;
            continue;
        }
        i += 1;
    }
    let seed_line = out
        .iter()
        .position(|(c, v)| code(c) == Some("9") && v.as_slice() == b"$HANDSEED")
        .expect("copied from the input")
        + 1;
    out[seed_line].1 = format!("{seed:X}").into_bytes();

    let mut fixed = Vec::with_capacity(bytes.len() + 32 * 12);
    for (c, v) in out {
        fixed.extend_from_slice(&c);
        fixed.extend_from_slice(EOL);
        fixed.extend_from_slice(&v);
        fixed.extend_from_slice(EOL);
    }
    Ok(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_numbers_have_only_the_decimals_they_need() {
        assert_eq!(mm_text(Length::from_mm(12)), "12");
        assert_eq!(mm_text(Length::from_nm(9_525_000)), "9.525");
        assert_eq!(mm_text(Length::from_nm(-1_500_000)), "-1.5");
    }

    #[test]
    fn file_names_avoid_characters_systems_reject() {
        assert_eq!(safe_file_name("B30 - Door 1"), "B30 - Door 1");
        assert_eq!(safe_file_name("W/30: \"top\"?."), "W_30_ _top__");
    }

    #[test]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "micrometers of a test part"
    )]
    fn grooves_along_a_part_stop_at_its_edges() {
        // Along X, 12 wide, centered 12.35 from the edge, past both ends.
        let outline = groove_outline((-1.0, 12.35), (801.0, 12.35), 12.0, (800.0, 600.0));
        let rounded: Vec<(i64, i64)> = outline
            .iter()
            .map(|&(x, y)| ((x * 1000.0).round() as i64, (y * 1000.0).round() as i64))
            .collect();
        assert_eq!(
            rounded,
            [(0, 6350), (800_000, 6350), (800_000, 18350), (0, 18350)]
        );
    }
}
