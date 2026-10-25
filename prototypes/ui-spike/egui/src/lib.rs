// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The egui front end as a library, so tests can drive it headlessly (see
//! `tests/edit.rs`).

use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape as EguiShape, Stroke, StrokeKind, Vec2,
};
use egui_extras::{Column, TableBuilder};
use oc_ui_core::drawing::{Drawing, Shape, ShapeKind};
use oc_ui_core::length::Length;
use oc_ui_core::{Params, Session, View};

const INK: Color32 = Color32::from_rgb(0x21, 0x21, 0x21);
const DIMENSION: Color32 = Color32::from_rgb(0x15, 0x65, 0xc0);

pub struct App {
    session: Session,
    view: View,
    width: String,
    height: String,
    depth: String,
    doors: u32,
    error: Option<String>,
    first_frame_reported: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// # Panics
    /// If the embedded fixture does not evaluate, which is a build error.
    #[must_use]
    pub fn new() -> Self {
        let session = Session::new();
        let params = session.params();
        let inches = |l: Length| {
            l.to_fractional_inches(32)
                .to_string()
                .trim_end_matches('"')
                .to_owned()
        };
        Self {
            view: session.view().expect("fixture evaluates"),
            width: inches(params.width),
            height: inches(params.height),
            depth: inches(params.depth),
            doors: params.doors,
            session,
            error: None,
            first_frame_reported: false,
        }
    }

    /// What the window currently shows.
    #[must_use]
    pub fn view(&self) -> &View {
        &self.view
    }

    /// The validation message shown under the fields, if any.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn apply(&mut self) {
        let parse = |label: &str, text: &str| {
            Length::parse_inches(text).ok_or_else(|| format!("{label}: cannot read \"{text}\""))
        };
        let result = (|| {
            let params = Params {
                width: parse("Width", &self.width)?,
                height: parse("Height", &self.height)?,
                depth: parse("Depth", &self.depth)?,
                doors: self.doors,
            };
            self.session.apply(params).map_err(|e| e.to_string())
        })();
        match result {
            Ok(view) => {
                self.view = view;
                self.error = None;
            }
            Err(message) => self.error = Some(message),
        }
    }

    fn parameters(&mut self, ui: &mut egui::Ui) {
        ui.heading("Base cabinet");
        ui.add_space(8.0);
        let mut changed = false;
        for (label, value) in [
            ("Width (in)", &mut self.width),
            ("Height (in)", &mut self.height),
            ("Depth (in)", &mut self.depth),
        ] {
            let label = ui.label(label);
            changed |= ui
                .text_edit_singleline(value)
                .labelled_by(label.id)
                .changed();
            ui.add_space(6.0);
        }
        let label = ui.label("Doors");
        changed |= ui
            .add(egui::DragValue::new(&mut self.doors).range(1..=2))
            .labelled_by(label.id)
            .changed();
        if changed {
            self.apply();
        }
        if let Some(error) = &self.error {
            ui.add_space(8.0);
            ui.colored_label(Color32::from_rgb(0xc6, 0x28, 0x28), error);
        }
    }

    fn cut_list(&self, ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::remainder().at_least(150.0))
            .column(Column::remainder().at_least(170.0))
            .columns(Column::auto().at_least(40.0), 1)
            .columns(Column::remainder().at_least(80.0), 3)
            .header(24.0, |mut header| {
                for title in ["Part", "Material", "Qty", "Length", "Width", "Thickness"] {
                    header.col(|ui| {
                        ui.strong(title);
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, self.view.cut_list.len(), |mut row| {
                    let r = &self.view.cut_list[row.index()];
                    for cell in [
                        r.part.as_str(),
                        &r.material,
                        &r.qty.to_string(),
                        &r.length,
                        &r.width,
                        &r.thickness,
                    ] {
                        row.col(|ui| {
                            ui.label(cell);
                        });
                    }
                });
            });
    }
}

/// Maps drawing millimeters (+y up) to screen points (+y down), fitted and centered.
struct Transform {
    scale: f32,
    origin: Pos2,
}

impl Transform {
    #[allow(clippy::cast_possible_truncation)]
    fn fit(drawing: &Drawing, area: Rect) -> Self {
        let span = Vec2::new(
            (drawing.max_x - drawing.min_x) as f32,
            (drawing.max_y - drawing.min_y) as f32,
        );
        let scale = (area.width() / span.x).min(area.height() / span.y);
        let padding = (area.size() - span * scale) / 2.0;
        let origin = Pos2::new(
            area.left() + padding.x - drawing.min_x as f32 * scale,
            area.top() + padding.y + drawing.max_y as f32 * scale,
        );
        Self { scale, origin }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn point(&self, x: f64, y: f64) -> Pos2 {
        Pos2::new(
            self.origin.x + x as f32 * self.scale,
            self.origin.y - y as f32 * self.scale,
        )
    }

    #[allow(clippy::cast_possible_truncation)]
    fn length(&self, mm: f64) -> f32 {
        mm as f32 * self.scale
    }
}

fn paint(ui: &egui::Ui, drawing: &Drawing, area: Rect) {
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::WHITE);
    let t = Transform::fit(drawing, area);
    let font = FontId::proportional(13.0);

    for shape in &drawing.shapes {
        match shape {
            Shape::Rect {
                kind,
                x,
                y,
                width,
                height,
            } => {
                let fill = match kind {
                    ShapeKind::ToeKick => Color32::from_gray(0xe0),
                    ShapeKind::Door => Color32::from_rgb(0xf3, 0xe3, 0xc3),
                    _ => Color32::TRANSPARENT,
                };
                let rect = Rect::from_two_pos(t.point(*x, *y), t.point(x + width, y + height));
                painter.rect(rect, 0.0, fill, Stroke::new(1.0, INK), StrokeKind::Inside);
            }
            Shape::Circle { cx, cy, radius, .. } => {
                painter.circle_stroke(
                    t.point(*cx, *cy),
                    t.length(*radius),
                    Stroke::new(1.0, Color32::from_gray(0x75)),
                );
            }
            Shape::Line { x1, y1, x2, y2 } => {
                painter.line_segment(
                    [t.point(*x1, *y1), t.point(*x2, *y2)],
                    Stroke::new(1.0, DIMENSION),
                );
            }
            Shape::Label {
                x,
                y,
                text,
                vertical,
            } => {
                let center = t.point(*x, *y);
                if *vertical {
                    let galley = painter.layout_no_wrap(text.clone(), font.clone(), DIMENSION);
                    let size = galley.size();
                    // Rotating by -90° around the top-left corner: shift so the
                    // rotated text is centered on the anchor.
                    let pos = center + Vec2::new(-size.y / 2.0, size.x / 2.0);
                    painter.add(EguiShape::Text(
                        egui::epaint::TextShape::new(pos, galley, DIMENSION)
                            .with_angle(-std::f32::consts::FRAC_PI_2),
                    ));
                } else {
                    painter.text(center, Align2::CENTER_CENTER, text, font.clone(), DIMENSION);
                }
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("parameters")
            .exact_size(240.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                self.parameters(ui);
            });
        egui::Panel::bottom("cut_list")
            .exact_size(280.0)
            .show(ui, |ui| self.cut_list(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                paint(ui, &self.view.drawing, area);
            });

        // egui paints right after `ui`, so the first call means the first
        // frame is about to be presented.
        if !self.first_frame_reported {
            self.first_frame_reported = true;
            oc_ui_core::first_frame_presented();
        }
    }
}
