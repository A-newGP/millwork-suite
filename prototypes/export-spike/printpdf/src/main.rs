// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the sample job as a shop drawing PDF with printpdf: one page per
//! elevation, then the cut list. It uses the built-in Helvetica, which every
//! PDF reader provides, so no font is embedded.

use std::path::Path;

use oc_export_core::sheet::{self, Align, Fit, LETTER_LANDSCAPE, Mark, PAGE_MARGIN_MM, Sheet};
use printpdf::{
    BuiltinFont, Line, LinePoint, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions,
    Point, TextItem, TextMatrix,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: oc-export-printpdf <out-dir>")?;
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;

    let job = oc_export_core::job(&oc_export_core::sample_project())?;
    let mut pages = Vec::new();
    for sheet in sheet::elevations(&job) {
        let fit = Fit::new(&sheet, LETTER_LANDSCAPE, PAGE_MARGIN_MM);
        pages.push(page(&sheet, fit));
    }
    for sheet in sheet::cut_list_pages(&job, LETTER_LANDSCAPE, PAGE_MARGIN_MM) {
        pages.push(page(&sheet, Fit::IDENTITY));
    }

    let mut warnings = Vec::new();
    let pdf = PdfDocument::new(&job.name)
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    for warning in &warnings {
        eprintln!("warning: {warning:?}");
    }
    std::fs::write(out.join("printpdf-shop.pdf"), pdf)?;
    Ok(())
}

fn page(sheet: &Sheet, fit: Fit) -> PdfPage {
    // 0.25 mm, the thin pen of technical drawings.
    let mut ops = vec![Op::SetOutlineThickness {
        pt: Mm(0.25).into(),
    }];
    let mm = |(x, y): (f64, f64)| Point::new(Mm(x as f32), Mm(y as f32));
    let title_at = (PAGE_MARGIN_MM, LETTER_LANDSCAPE.1 - PAGE_MARGIN_MM);
    text(&mut ops, mm(title_at), 5.0, &sheet.title, false);

    for mark in &sheet.marks {
        match mark {
            Mark::Line { from, to, .. } => ops.push(Op::DrawLine {
                line: Line {
                    points: [fit.point(*from), fit.point(*to)]
                        .map(|p| LinePoint {
                            p: mm(p),
                            bezier: false,
                        })
                        .into(),
                    is_closed: false,
                },
            }),
            Mark::Circle { center, radius, .. } => {
                let (cx, cy) = fit.point(*center);
                ops.push(Op::DrawLine {
                    line: circle(cx, cy, radius * fit.scale),
                });
            }
            Mark::Text {
                at,
                height,
                text: value,
                align,
                vertical,
                ..
            } => {
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
                text(&mut ops, mm((x, y)), size, value, *vertical);
            }
        }
    }
    let (w, h) = LETTER_LANDSCAPE;
    PdfPage::new(Mm(w as f32), Mm(h as f32), ops)
}

/// Text with its baseline start at `at`; `size` is the height in millimeters.
fn text(ops: &mut Vec<Op>, at: Point, size: f64, value: &str, vertical: bool) {
    // The text matrix: identity, or a quarter turn counterclockwise.
    let (a, b, c, d) = if vertical {
        (0.0, 1.0, -1.0, 0.0)
    } else {
        (1.0, 0.0, 0.0, 1.0)
    };
    ops.extend([
        Op::StartTextSection,
        Op::SetTextMatrix {
            matrix: TextMatrix::Raw([a, b, c, d, at.x.0, at.y.0]),
        },
        Op::SetFont {
            font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
            size: Mm(size as f32).into(),
        },
        Op::ShowText {
            items: vec![TextItem::Text(value.to_owned())],
        },
        Op::EndTextSection,
    ]);
}

/// A circle as four cubic Bézier arcs: each arc is two control points
/// (`bezier: true`) and an end point.
fn circle(cx: f64, cy: f64, r: f64) -> Line {
    const K: f64 = 0.552_284_75;
    let k = K * r;
    let at = |x: f64, y: f64, bezier| LinePoint {
        p: Point::new(Mm(x as f32), Mm(y as f32)),
        bezier,
    };
    Line {
        points: vec![
            at(cx + r, cy, false),
            at(cx + r, cy + k, true),
            at(cx + k, cy + r, true),
            at(cx, cy + r, false),
            at(cx - k, cy + r, true),
            at(cx - r, cy + k, true),
            at(cx - r, cy, false),
            at(cx - r, cy - k, true),
            at(cx - k, cy - r, true),
            at(cx, cy - r, false),
            at(cx + k, cy - r, true),
            at(cx + r, cy - k, true),
            at(cx + r, cy, false),
        ],
        is_closed: true,
    }
}
