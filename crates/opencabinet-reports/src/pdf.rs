// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The cut list as a PDF to print (ADR-0015): landscape pages with a title,
//! the problems found, and a table that continues across pages.

// Page layout works in floating point millimeters; the counts converted
// (rows, problems) are small and the results are positions on paper.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "page layout in floating point millimeters"
)]

use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point};
use krilla::metadata::Metadata;
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use opencabinet_model::Units;
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};

use crate::cut_list::{CutList, CutRow};

/// A sheet of paper, in millimeters, portrait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    /// US Letter, 8-1/2" × 11".
    Letter,
    /// ISO A4, 210 × 297 mm.
    A4,
}

impl Paper {
    /// The usual paper for a project's units: Letter for inches, A4 for
    /// millimeters.
    #[must_use]
    pub fn for_units(units: Units) -> Self {
        match units {
            Units::Imperial => Self::Letter,
            Units::Metric => Self::A4,
        }
    }

    /// Width and height in millimeters, landscape.
    fn landscape(self) -> (f32, f32) {
        match self {
            Self::Letter => (279.4, 215.9),
            Self::A4 => (297.0, 210.0),
        }
    }
}

/// The two faces of the bundled font the PDF uses.
pub struct Fonts {
    regular: Face,
    bold: Face,
}

impl Fonts {
    /// Reads the regular and bold faces from their TrueType files' bytes.
    ///
    /// # Errors
    /// When either is not a font.
    pub fn new(regular: Vec<u8>, bold: Vec<u8>) -> Result<Self, String> {
        Ok(Self {
            regular: Face::new(regular)?,
            bold: Face::new(bold)?,
        })
    }
}

/// A font face: for drawing, and for measuring text.
struct Face {
    bytes: Vec<u8>,
    font: Font,
}

impl Face {
    fn new(bytes: Vec<u8>) -> Result<Self, String> {
        FontRef::new(&bytes).map_err(|_| "not a font".to_owned())?;
        let font = Font::new(bytes.clone().into(), 0).ok_or("not a font")?;
        Ok(Self { bytes, font })
    }

    /// The width of `text` at `size` points, from the font's own metrics.
    fn width(&self, text: &str, size: f32) -> f32 {
        let font = FontRef::new(&self.bytes).expect("checked when loaded");
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(Size::new(size), LocationRef::default());
        text.chars()
            .map(|c| {
                charmap
                    .map(c)
                    .and_then(|g| metrics.advance_width(g))
                    .unwrap_or(size / 2.0)
            })
            .sum()
    }
}

const PT_PER_MM: f32 = 72.0 / 25.4;
const MARGIN: f32 = 12.7;
const TITLE_SIZE: f32 = 14.0;
const TEXT_SIZE: f32 = 8.5;
const ROW: f32 = 6.0;
/// Each column's share of the table's width, in the order of
/// [`CutRow::TITLES`].
const COLUMNS: [f32; 10] = [0.07, 0.15, 0.17, 0.04, 0.09, 0.09, 0.07, 0.10, 0.10, 0.12];

/// The cut list as PDF bytes. `title` heads every page and `generator`,
/// such as "OpenCabinet Designer 0.1.0", is written in its footer and
/// metadata. The same input always gives the same bytes.
///
/// # Errors
/// When the PDF cannot be written.
pub fn cut_list_pdf(
    list: &CutList,
    title: &str,
    generator: &str,
    paper: Paper,
    fonts: &Fonts,
) -> Result<Vec<u8>, String> {
    let (width, height) = paper.landscape();
    let table_top = MARGIN + 12.0 + list.problems.len() as f32 * 4.5;
    let rows_per_page = (((height - MARGIN - 8.0 - table_top) / ROW) as usize)
        .saturating_sub(1)
        .max(1);
    let chunks: Vec<&[CutRow]> = if list.rows.is_empty() {
        vec![&[]]
    } else {
        list.rows.chunks(rows_per_page).collect()
    };

    let mut document = Document::new();
    document.set_metadata(
        Metadata::new()
            .title(format!("Cut list — {title}"))
            .creator(generator.to_owned()),
    );
    let pages = chunks.len();
    for (index, rows) in chunks.into_iter().enumerate() {
        let settings = PageSettings::from_wh(width * PT_PER_MM, height * PT_PER_MM)
            .ok_or("the paper size is not positive")?;
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        let mut pen = Pen {
            surface: &mut surface,
        };

        pen.text(
            &fonts.bold,
            (MARGIN, MARGIN + 5.0),
            TITLE_SIZE,
            &format!("Cut list — {title}"),
        );
        let page_label = format!("Page {} of {pages}", index + 1);
        let label_width = fonts.regular.width(&page_label, TEXT_SIZE) / PT_PER_MM;
        pen.text(
            &fonts.regular,
            (width - MARGIN - label_width, MARGIN + 5.0),
            TEXT_SIZE,
            &page_label,
        );
        for (i, problem) in list.problems.iter().enumerate() {
            let y = MARGIN + 11.0 + i as f32 * 4.5;
            pen.text(&fonts.regular, (MARGIN, y), TEXT_SIZE, problem);
        }

        let table_width = width - 2.0 * MARGIN;
        let header = CutRow::TITLES;
        pen.row(&fonts.bold, table_top, table_width, &header);
        pen.rule(table_top + ROW, table_width, 0.4);
        for (i, row) in rows.iter().enumerate() {
            let y = table_top + ROW * (i as f32 + 1.0);
            pen.row(&fonts.regular, y, table_width, &row.cells());
            pen.rule(y + ROW, table_width, 0.15);
        }
        pen.text(
            &fonts.regular,
            (MARGIN, height - MARGIN + 4.0),
            TEXT_SIZE - 1.5,
            generator,
        );

        surface.finish();
        page.finish();
    }
    document
        .finish()
        .map_err(|e| format!("Cannot write the PDF: {e:?}"))
}

/// Draws on a page in millimeters from the top-left corner.
struct Pen<'s, 'p> {
    surface: &'s mut Surface<'p>,
}

impl Pen<'_, '_> {
    /// `text` with its baseline starting at `at`.
    fn text(&mut self, face: &Face, at: (f32, f32), size: f32, text: &str) {
        self.surface.set_fill(Some(Fill::default()));
        self.surface.set_stroke(None);
        self.surface.draw_text(
            Point::from_xy(at.0 * PT_PER_MM, at.1 * PT_PER_MM),
            face.font.clone(),
            size,
            text,
            false,
            TextDirection::Auto,
        );
    }

    /// One table row whose top is at `top`; each cell is cut to its column,
    /// with an ellipsis, measured with the font.
    fn row(&mut self, face: &Face, top: f32, width: f32, cells: &[&str]) {
        let mut x = MARGIN;
        for (cell, share) in cells.iter().zip(COLUMNS) {
            let column = share * width;
            let room = (column - 2.0) * PT_PER_MM;
            let text = fit(face, cell, TEXT_SIZE, room);
            self.text(face, (x + 1.0, top + ROW - 1.8), TEXT_SIZE, &text);
            x += column;
        }
    }

    /// A horizontal line under a row, `weight` points thick.
    fn rule(&mut self, y: f32, width: f32, weight: f32) {
        let mut path = PathBuilder::new();
        path.move_to(MARGIN * PT_PER_MM, y * PT_PER_MM);
        path.line_to((MARGIN + width) * PT_PER_MM, y * PT_PER_MM);
        let Some(path) = path.finish() else { return };
        self.surface.set_fill(None);
        self.surface.set_stroke(Some(Stroke {
            paint: rgb::Color::new(140, 140, 140).into(),
            width: weight,
            ..Stroke::default()
        }));
        self.surface.draw_path(&path);
    }
}

/// `text`, shortened with an ellipsis until it fits in `room` points.
fn fit(face: &Face, text: &str, size: f32, room: f32) -> String {
    if face.width(text, size) <= room {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if face.width(&candidate, size) <= room {
            return candidate;
        }
    }
    String::new()
}
