// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Throwaway UI spike core: the stack spike's evaluator plus an elevation
//! drawing and an editing session. Every front end uses this crate, so the
//! comparison measures the user interface toolkits, not the geometry.

pub mod drawing;
pub mod error;
pub mod eval;
pub mod length;
pub mod model;

use std::io::Write;
use std::time::Duration;

use serde::Serialize;

use crate::drawing::Drawing;
use crate::error::{Error, Result};
use crate::length::Length;
use crate::model::{Insert, Project};

/// The editable parameters of the single cabinet in the spike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    pub width: Length,
    pub height: Length,
    pub depth: Length,
    pub doors: u32,
}

/// One row of the cut list, formatted for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CutRow {
    pub part: String,
    pub material: String,
    pub qty: u32,
    pub length: String,
    pub width: String,
    pub thickness: String,
}

/// Everything a front end shows: the drawing and the cut list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub drawing: Drawing,
    pub cut_list: Vec<CutRow>,
}

/// Holds the project being edited: the spike fixture's first cabinet.
pub struct Session {
    project: Project,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// # Panics
    /// If the embedded fixture is invalid, which is a build error.
    #[must_use]
    pub fn new() -> Self {
        let mut project: Project = serde_json::from_str(include_str!("../assets/project.json"))
            .expect("embedded fixture is valid");
        project.products.truncate(1);
        Self { project }
    }

    #[must_use]
    pub fn params(&self) -> Params {
        let product = &self.project.products[0];
        let Insert::Doors { count, .. } = product.front.insert;
        Params {
            width: product.width,
            height: product.height,
            depth: product.depth,
            doors: count,
        }
    }

    /// Applies new parameters if the cabinet they describe can be evaluated,
    /// and returns the new view. On error the session is left unchanged.
    ///
    /// # Errors
    /// When the parameters describe a cabinet that cannot be built.
    pub fn apply(&mut self, params: Params) -> Result<View> {
        let mut candidate = self.project.clone();
        let product = &mut candidate.products[0];
        product.width = params.width;
        product.height = params.height;
        product.depth = params.depth;
        let Insert::Doors { count, .. } = &mut product.front.insert;
        *count = params.doors;

        let view = view_of(&candidate)?;
        self.project = candidate;
        Ok(view)
    }

    /// # Errors
    /// Never for the embedded fixture; kept fallible for symmetry with `apply`.
    pub fn view(&self) -> Result<View> {
        view_of(&self.project)
    }
}

fn view_of(project: &Project) -> Result<View> {
    let evaluation = eval::evaluate(project)?;
    let product = &project.products[0];
    let standard = project
        .library
        .standards
        .iter()
        .find(|s| s.id == product.standard)
        .ok_or_else(|| Error::InvalidProject("unknown standard".into()))?;

    let inches = |l: Length| l.to_fractional_inches(32).to_string();
    let cut_list = evaluation
        .parts
        .iter()
        .map(|p| CutRow {
            part: p.key.clone(),
            material: p.material.name.clone(),
            qty: p.quantity,
            length: inches(p.length),
            width: inches(p.width),
            thickness: inches(p.thickness()),
        })
        .collect();
    Ok(View {
        drawing: drawing::elevation(product, standard, &evaluation),
        cut_list,
    })
}

/// Measurement support shared by every front end: with `OC_MEASURE` set, a
/// front end calls this once its first frame is on screen. It prints a marker
/// the measurement script waits for, then exits after a pause long enough to
/// sample memory.
pub fn first_frame_presented() {
    if std::env::var_os("OC_MEASURE").is_none() {
        return;
    }
    let mut stdout = std::io::stdout();
    writeln!(stdout, "FIRST_FRAME").ok();
    stdout.flush().ok();
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(3));
        std::process::exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_contains_parts_doors_and_dimensions() {
        let view = Session::new().view().unwrap();
        assert_eq!(view.cut_list.len(), 8);
        let doors = view
            .drawing
            .shapes
            .iter()
            .filter(|s| {
                matches!(
                    s,
                    drawing::Shape::Rect {
                        kind: drawing::ShapeKind::Door,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(doors, 2);
    }

    #[test]
    fn rejects_impossible_cabinets_and_keeps_state() {
        let mut session = Session::new();
        let before = session.params();
        let tiny = Params {
            width: Length::from_nm(10_000_000),
            ..before
        };
        assert!(session.apply(tiny).is_err());
        assert_eq!(session.params(), before);
    }
}
