// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the sample job as DXF 2018 with the `dxf` crate, which handles the
//! handles, tables and objects a modern DXF needs, and can declare units.

use std::path::Path;

use dxf::entities::{Circle, Entity, EntityType, Line, Text};
use dxf::enums::{AcadVersion, HorizontalTextJustification, Units, VerticalTextJustification};
use dxf::tables::Layer as DxfLayer;
use dxf::{Color, Drawing, Point};
use oc_export_core::sheet::{self, Align, Layer, Mark, Sheet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: oc-export-dxf <out-dir>")?;
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;

    let job = oc_export_core::job(&oc_export_core::sample_project())?;
    drawing(&sheet::elevations_side_by_side(&job)).save_file(out.join("dxf-elevations.dxf"))?;
    drawing(&sheet::machining(&job)).save_file(out.join("dxf-machining.dxf"))?;
    Ok(())
}

/// DXF transparency: flag 0x02000000 plus alpha 255.
const OPAQUE: i32 = 0x0200_00FF;

fn drawing(sheet: &Sheet) -> Drawing {
    let mut drawing = Drawing::new();
    drawing.header.version = AcadVersion::R2018;
    drawing.header.default_drawing_units = Units::Millimeters;
    for layer in Layer::ALL {
        drawing.add_layer(DxfLayer {
            name: layer.name().into(),
            color: Color::from_index(layer.color_index()),
            ..Default::default()
        });
    }

    let point = |(x, y): (f64, f64)| Point::new(x, y, 0.0);
    for mark in &sheet.marks {
        let (layer, specific) = match mark {
            Mark::Line { layer, from, to } => (
                layer,
                EntityType::Line(Line {
                    p1: point(*from),
                    p2: point(*to),
                    ..Default::default()
                }),
            ),
            Mark::Circle {
                layer,
                center,
                radius,
            } => (
                layer,
                EntityType::Circle(Circle {
                    center: point(*center),
                    radius: *radius,
                    ..Default::default()
                }),
            ),
            Mark::Text {
                layer,
                at,
                height,
                text,
                align,
                vertical,
            } => {
                let mut entity = Text {
                    location: point(*at),
                    text_height: *height,
                    value: text.clone(),
                    rotation: if *vertical { 90.0 } else { 0.0 },
                    ..Default::default()
                };
                if *align == Align::Center {
                    entity.horizontal_text_justification = HorizontalTextJustification::Center;
                    entity.vertical_text_justification = VerticalTextJustification::Middle;
                    entity.second_alignment_point = point(*at);
                }
                (layer, EntityType::Text(entity))
            }
        };
        let mut entity = Entity::new(specific);
        entity.common.layer = layer.name().into();
        // The crate's default, 0, is not a valid transparency; write opaque.
        entity.common.transparency = OPAQUE;
        drawing.add_entity(entity);
    }
    drawing.normalize();
    drawing
}
