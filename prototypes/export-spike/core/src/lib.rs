// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Throwaway export spike core: the UI spike's evaluator and elevation
//! drawing, turned into a job that every exporter writes. Exporters only map
//! this data to their file format, so the comparison measures the libraries.

pub mod drawing;
pub mod error;
pub mod eval;
pub mod length;
pub mod model;
pub mod sheet;

use std::fmt::Write;

use crate::drawing::Drawing;
use crate::error::{Error, Result};
use crate::length::Length;
use crate::model::Project;

/// Display precision for the shop: 1/32".
const DENOMINATOR: i64 = 32;

/// Everything the exporters write for one project.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub name: String,
    /// One front elevation per product.
    pub elevations: Vec<Elevation>,
    pub cut_list: Vec<CutRow>,
    /// Parts with machining, in part coordinates, for CNC output.
    pub machined: Vec<MachinedPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Elevation {
    pub label: String,
    pub drawing: Drawing,
}

/// One row of the cut list, formatted for the shop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutRow {
    pub product: String,
    pub part: String,
    pub material: String,
    pub qty: u32,
    pub length: String,
    pub width: String,
    pub thickness: String,
}

impl CutRow {
    pub const HEADERS: [&str; 7] = [
        "Product", "Part", "Material", "Qty", "Length", "Width", "Thick.",
    ];

    pub fn cells(&self) -> [String; 7] {
        [
            self.product.clone(),
            self.part.clone(),
            self.material.clone(),
            self.qty.to_string(),
            self.length.clone(),
            self.width.clone(),
            self.thickness.clone(),
        ]
    }
}

/// A part outline with its holes, in millimeters: x along the length, y
/// across the width, origin at a corner of face A.
#[derive(Debug, Clone, PartialEq)]
pub struct MachinedPart {
    pub name: String,
    pub length: f64,
    pub width: f64,
    pub holes: Vec<MachinedHole>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MachinedHole {
    pub x: f64,
    pub y: f64,
    pub diameter: f64,
    pub depth: f64,
}

/// The spike fixture: four cabinets from the stack spike.
///
/// # Panics
/// If the embedded fixture is invalid, which is a build error.
pub fn sample_project() -> Project {
    serde_json::from_str(include_str!("../assets/project.json")).expect("embedded fixture is valid")
}

/// Evaluates a project into everything the exporters need.
///
/// # Errors
/// When a product cannot be evaluated.
pub fn job(project: &Project) -> Result<Job> {
    let evaluation = eval::evaluate(project)?;
    let inches = |l: Length| l.to_fractional_inches(DENOMINATOR).to_string();

    let mut elevations = Vec::new();
    for product in &project.products {
        let standard = project
            .library
            .standards
            .iter()
            .find(|s| s.id == product.standard)
            .ok_or_else(|| Error::InvalidProject(format!("{}: unknown standard", product.label)))?;
        let parts = eval::Evaluation {
            parts: evaluation
                .parts
                .iter()
                .filter(|p| p.product == product.label)
                .cloned()
                .collect(),
            holes: evaluation
                .holes
                .iter()
                .filter(|h| h.product == product.label)
                .cloned()
                .collect(),
        };
        elevations.push(Elevation {
            label: product.label.clone(),
            drawing: drawing::elevation(product, standard, &parts),
        });
    }

    let cut_list = evaluation
        .parts
        .iter()
        .map(|p| CutRow {
            product: p.product.to_owned(),
            part: p.key.clone(),
            material: p.material.name.clone(),
            qty: p.quantity,
            length: inches(p.length),
            width: inches(p.width),
            thickness: inches(p.thickness()),
        })
        .collect();

    let mm = Length::to_mm_f64;
    let machined = evaluation
        .parts
        .iter()
        .filter_map(|p| {
            let holes: Vec<_> = evaluation
                .holes
                .iter()
                .filter(|h| h.product == p.product && h.part == p.key)
                .map(|h| MachinedHole {
                    x: mm(h.x),
                    y: mm(h.y),
                    diameter: mm(h.diameter),
                    depth: mm(h.depth),
                })
                .collect();
            (!holes.is_empty()).then(|| MachinedPart {
                name: format!("{} {}", p.product, p.key),
                length: mm(p.length),
                width: mm(p.width),
                holes,
            })
        })
        .collect();

    Ok(Job {
        name: project.name.clone(),
        elevations,
        cut_list,
        machined,
    })
}

/// The cut list as CSV (RFC 4180): comma separated, CRLF line ends, fields
/// quoted when they contain a comma, a quote or a line break. Inch marks make
/// quoting necessary in almost every row.
pub fn cut_list_csv(rows: &[CutRow]) -> String {
    let mut csv = String::new();
    let mut line = |cells: &[String]| {
        let quoted: Vec<String> = cells.iter().map(|c| csv_field(c)).collect();
        write!(csv, "{}\r\n", quoted.join(",")).expect("writing to a String");
    };
    line(&CutRow::HEADERS.map(String::from));
    for row in rows {
        line(&row.cells());
    }
    csv
}

fn csv_field(cell: &str) -> String {
    if cell.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_has_an_elevation_per_product_and_machined_doors() {
        let job = job(&sample_project()).unwrap();
        assert_eq!(job.elevations.len(), 4);
        assert!(job.cut_list.len() > 20);
        assert!(job.machined.iter().all(|p| p.name.contains("door")));
        assert!(!job.machined.is_empty());
    }

    #[test]
    fn csv_quotes_inch_marks() {
        let row = CutRow {
            product: "B30".into(),
            part: "case/bottom".into(),
            material: "Plywood, 3/4\"".into(),
            qty: 1,
            length: "28-1/2\"".into(),
            width: "24\"".into(),
            thickness: "23/32\"".into(),
        };
        let csv = cut_list_csv(&[row]);
        assert_eq!(
            csv.lines().nth(1).unwrap(),
            "B30,case/bottom,\"Plywood, 3/4\"\"\",1,\"28-1/2\"\"\",\"24\"\"\",\"23/32\"\"\""
        );
    }
}
