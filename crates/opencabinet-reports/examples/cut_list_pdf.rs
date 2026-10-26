// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the sample kitchen's cut list as a PDF, so CI can check it with
//! independent readers.
//!
//! `cargo run -p opencabinet-reports --example cut_list_pdf -- <out.pdf>`

use std::path::PathBuf;

use opencabinet_model::{FrontStyle, Length, Library, Project, Units};
use opencabinet_reports::{Fonts, Paper, cut_list, cut_list_pdf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: cut_list_pdf <out.pdf>")?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = root.join("fixtures/kitchen.ocab");
    let mut project: Project =
        serde_json::from_str(&std::fs::read_to_string(fixture.join("project.json"))?)?;
    let library: Library =
        serde_json::from_str(&std::fs::read_to_string(fixture.join("library.json"))?)?;
    // Five-piece doors are not evaluated yet; show the base cabinet's parts.
    if let Some(front) = project.products[0].front.as_mut() {
        front.children[1].insert.as_mut().ok_or("no insert")?.style = Some(FrontStyle::Slab);
    }
    let fonts = root.join("assets/fonts/noto-sans");
    let fonts = Fonts::new(
        std::fs::read(fonts.join("NotoSans-Regular.ttf"))?,
        std::fs::read(fonts.join("NotoSans-Bold.ttf"))?,
    )?;
    let list = cut_list(
        &project,
        &library,
        Units::Imperial,
        Length::from_inch_fraction(1, 32).ok_or("fraction")?,
    );
    let pdf = cut_list_pdf(
        &list,
        &project.name,
        "OpenCabinet example",
        Paper::Letter,
        &fonts,
    )?;
    std::fs::write(out, pdf)?;
    Ok(())
}
