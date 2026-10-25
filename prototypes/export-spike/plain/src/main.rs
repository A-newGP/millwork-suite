// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the sample job as DXF R12 and CSV with the standard library only.
//!
//! R12 is the oldest DXF version every CAD program still reads, and the only
//! one simple enough to write by hand: later versions require handles,
//! object dictionaries and class tables. Its cost: R12 cannot declare its
//! units, so the reader must be told the drawing is in millimeters.

use std::fmt::Write;
use std::path::Path;

use oc_export_core::sheet::{self, Align, Layer, Mark, Sheet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: oc-export-plain <out-dir>")?;
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;

    let job = oc_export_core::job(&oc_export_core::sample_project())?;
    std::fs::write(
        out.join("plain-elevations.dxf"),
        dxf(&sheet::elevations_side_by_side(&job)),
    )?;
    std::fs::write(
        out.join("plain-machining.dxf"),
        dxf(&sheet::machining(&job)),
    )?;
    std::fs::write(
        out.join("cut-list.csv"),
        oc_export_core::cut_list_csv(&job.cut_list),
    )?;
    Ok(())
}

fn dxf(sheet: &Sheet) -> String {
    let mut w = Writer(String::new());
    w.section("HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1009");
    w.pair(9, "$EXTMIN");
    w.point(10, sheet.min);
    w.pair(9, "$EXTMAX");
    w.point(10, sheet.max);
    w.end_section();

    w.section("TABLES");
    w.table("LTYPE", 1);
    w.pair(0, "LTYPE");
    w.pair(2, "CONTINUOUS");
    w.pair(70, 0);
    w.pair(3, "Solid line");
    w.pair(72, 65);
    w.pair(73, 0);
    w.pair(40, 0.0);
    w.pair(0, "ENDTAB");
    w.table("LAYER", Layer::ALL.len());
    for layer in Layer::ALL {
        w.pair(0, "LAYER");
        w.pair(2, layer.name());
        w.pair(70, 0);
        w.pair(62, layer.color_index());
        w.pair(6, "CONTINUOUS");
    }
    w.pair(0, "ENDTAB");
    w.end_section();

    w.section("ENTITIES");
    for mark in &sheet.marks {
        match mark {
            Mark::Line { layer, from, to } => {
                w.entity("LINE", *layer);
                w.point(10, *from);
                w.point(11, *to);
            }
            Mark::Circle {
                layer,
                center,
                radius,
            } => {
                w.entity("CIRCLE", *layer);
                w.point(10, *center);
                w.pair(40, *radius);
            }
            Mark::Text {
                layer,
                at,
                height,
                text,
                align,
                vertical,
            } => {
                w.entity("TEXT", *layer);
                w.point(10, *at);
                w.pair(40, *height);
                w.pair(1, escape(text));
                if *vertical {
                    w.pair(50, 90.0);
                }
                if *align == Align::Center {
                    // Middle-center: the alignment point (11) is the anchor.
                    w.pair(72, 1);
                    w.point(11, *at);
                    w.pair(73, 2);
                }
            }
        }
    }
    w.end_section();
    w.pair(0, "EOF");
    w.0
}

/// DXF is a list of group code / value pairs, one per line.
struct Writer(String);

impl Writer {
    fn pair(&mut self, code: u16, value: impl std::fmt::Display) {
        write!(self.0, "{code:>3}\r\n{value}\r\n").expect("writing to a String");
    }

    /// A 2D point as its x (`code`), y (`code + 10`) and z (`code + 20`).
    fn point(&mut self, code: u16, (x, y): (f64, f64)) {
        self.pair(code, x);
        self.pair(code + 10, y);
        self.pair(code + 20, 0.0);
    }

    fn section(&mut self, name: &str) {
        self.pair(0, "SECTION");
        self.pair(2, name);
    }

    fn end_section(&mut self) {
        self.pair(0, "ENDSEC");
    }

    fn table(&mut self, name: &str, entries: usize) {
        self.pair(0, "TABLE");
        self.pair(2, name);
        self.pair(70, entries);
    }

    fn entity(&mut self, kind: &str, layer: Layer) {
        self.pair(0, kind);
        self.pair(8, layer.name());
    }
}

/// R12 text is 8-bit; anything outside ASCII uses the `\U+XXXX` escape that
/// CAD programs understand.
fn escape(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_string()
            } else {
                format!("\\U+{:04X}", u32::from(c))
            }
        })
        .collect()
}
