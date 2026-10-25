// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the sample job as a shop drawing PDF with krilla: one page per
//! elevation, then the cut list. krilla embeds a subset of a TrueType font,
//! so it needs one: `OC_FONT`, or a common system font.

use std::path::{Path, PathBuf};

use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Transform};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::text::{Font, TextDirection};
use oc_export_core::sheet::{self, Align, Fit, LETTER_LANDSCAPE, Mark, PAGE_MARGIN_MM, Sheet};

const PT_PER_MM: f64 = 72.0 / 25.4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: oc-export-krilla <out-dir>")?;
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;
    let font_path = font_path().ok_or("no font found; set OC_FONT to a .ttf file")?;
    let font = Font::new(std::fs::read(&font_path)?.into(), 0).ok_or("unreadable font")?;

    let job = oc_export_core::job(&oc_export_core::sample_project())?;
    let mut document = Document::new();
    for sheet in sheet::elevations(&job) {
        let fit = Fit::new(&sheet, LETTER_LANDSCAPE, PAGE_MARGIN_MM);
        page(&mut document, &font, &sheet, fit);
    }
    for sheet in sheet::cut_list_pages(&job, LETTER_LANDSCAPE, PAGE_MARGIN_MM) {
        page(&mut document, &font, &sheet, Fit::IDENTITY);
    }
    let pdf = document.finish().map_err(|e| format!("PDF error: {e:?}"))?;
    std::fs::write(out.join("krilla-shop.pdf"), pdf)?;
    Ok(())
}

fn page(document: &mut Document, font: &Font, sheet: &Sheet, fit: Fit) {
    let (w, h) = (
        LETTER_LANDSCAPE.0 * PT_PER_MM,
        LETTER_LANDSCAPE.1 * PT_PER_MM,
    );
    let mut page =
        document.start_page_with(PageSettings::from_wh(w as f32, h as f32).expect("positive"));
    let mut surface = page.surface();
    // PDF space as krilla presents it has +y down; flip it so sheet
    // millimeters with +y up map directly.
    surface.push_transform(&Transform::from_row(
        PT_PER_MM as f32,
        0.0,
        0.0,
        -PT_PER_MM as f32,
        0.0,
        h as f32,
    ));
    surface.set_stroke(Some(Stroke {
        paint: rgb::Color::black().into(),
        // In millimeters, like everything under the flip transform: the thin
        // pen of technical drawings.
        width: 0.25,
        ..Default::default()
    }));
    surface.set_fill(None);

    let mut path = PathBuilder::new();
    let mut texts = Vec::new();
    for mark in &sheet.marks {
        match mark {
            Mark::Line { from, to, .. } => {
                let ((x1, y1), (x2, y2)) = (fit.point(*from), fit.point(*to));
                path.move_to(x1 as f32, y1 as f32);
                path.line_to(x2 as f32, y2 as f32);
            }
            Mark::Circle { center, radius, .. } => {
                let (cx, cy) = fit.point(*center);
                circle(&mut path, cx, cy, radius * fit.scale);
            }
            Mark::Text { .. } => texts.push(mark),
        }
    }
    if let Some(path) = path.finish() {
        surface.draw_path(&path);
    }

    surface.set_stroke(None);
    surface.set_fill(Some(Fill::default()));
    let title_at = (PAGE_MARGIN_MM, LETTER_LANDSCAPE.1 - PAGE_MARGIN_MM);
    text(&mut surface, font, title_at, 5.0, &sheet.title, false);
    for mark in texts {
        if let Mark::Text {
            at,
            height,
            text: value,
            align,
            vertical,
            ..
        } = mark
        {
            let size = height * fit.scale;
            let (mut x, mut y) = fit.point(*at);
            if *align == Align::Center {
                let half = sheet::approx_text_width(value, size) / 2.0;
                if *vertical {
                    (x, y) = (x + size / 2.0, y - half);
                } else {
                    (x, y) = (x - half, y - size / 2.0);
                }
            }
            text(&mut surface, font, (x, y), size, value, *vertical);
        }
    }
    surface.pop();
    surface.finish();
    page.finish();
}

/// Draws text with its baseline start at `at`, in page millimeters.
fn text(
    surface: &mut krilla::surface::Surface<'_>,
    font: &Font,
    at: (f64, f64),
    size: f64,
    value: &str,
    vertical: bool,
) {
    // Text must be drawn upright in a +y down space: undo the flip locally.
    let angle = if vertical { -90.0 } else { 0.0 };
    surface.push_transform(&Transform::from_row(
        1.0,
        0.0,
        0.0,
        -1.0,
        at.0 as f32,
        at.1 as f32,
    ));
    surface.push_transform(&Transform::from_rotate(angle));
    surface.draw_text(
        Point::from_xy(0.0, 0.0),
        font.clone(),
        size as f32,
        value,
        false,
        TextDirection::Auto,
    );
    surface.pop();
    surface.pop();
}

/// A circle as four cubic Bézier arcs; PDF has no circle operator.
fn circle(path: &mut PathBuilder, cx: f64, cy: f64, r: f64) {
    const K: f64 = 0.552_284_75;
    let (k, f) = (K * r, |v: f64| v as f32);
    path.move_to(f(cx + r), f(cy));
    path.cubic_to(f(cx + r), f(cy + k), f(cx + k), f(cy + r), f(cx), f(cy + r));
    path.cubic_to(f(cx - k), f(cy + r), f(cx - r), f(cy + k), f(cx - r), f(cy));
    path.cubic_to(f(cx - r), f(cy - k), f(cx - k), f(cy - r), f(cx), f(cy - r));
    path.cubic_to(f(cx + k), f(cy - r), f(cx + r), f(cy - k), f(cx + r), f(cy));
    path.close();
}

fn font_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("OC_FONT") {
        return Some(path.into());
    }
    [
        "C:/Windows/Fonts/arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
}
