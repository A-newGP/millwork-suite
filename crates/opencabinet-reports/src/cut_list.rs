// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The cut list as people read it: the evaluator's parts as rows of text in
//! the user's units, and what could not be evaluated.

use opencabinet_eval::{Part, evaluate};
use opencabinet_model::{Length, Library, Project, Units};

/// One row of the cut list, ready to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutRow {
    /// The product's label, such as "B30".
    pub product: String,
    /// The part's name, such as "Left side" or "Door 1".
    pub part: String,
    /// The material's name.
    pub material: String,
    /// How many to cut.
    pub quantity: String,
    /// The finished length, along the grain.
    pub length: String,
    /// The finished width.
    pub width: String,
    /// The material's actual thickness.
    pub thickness: String,
    /// The length to cut, before banding.
    pub cut_length: String,
    /// The width to cut, before banding.
    pub cut_width: String,
    /// How many edges are banded.
    pub edges: String,
}

impl CutRow {
    /// The column titles, in the order of [`CutRow::cells`].
    pub const TITLES: [&str; 10] = [
        "Product",
        "Part",
        "Material",
        "Qty",
        "Length",
        "Width",
        "Thick.",
        "Cut length",
        "Cut width",
        "Edges",
    ];

    /// The row's cells, in the order of [`CutRow::TITLES`].
    #[must_use]
    pub fn cells(&self) -> [&str; 10] {
        [
            &self.product,
            &self.part,
            &self.material,
            &self.quantity,
            &self.length,
            &self.width,
            &self.thickness,
            &self.cut_length,
            &self.cut_width,
            &self.edges,
        ]
    }
}

/// The rows of the cut list, and a sentence for each product that could
/// not be evaluated or has something worth knowing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CutList {
    /// One row per part, in product order.
    pub rows: Vec<CutRow>,
    /// Such as "B30: drawer boxes are not evaluated yet".
    pub problems: Vec<String>,
}

/// Evaluates the project and shows its parts in `units` at `precision`.
#[must_use]
pub fn cut_list(project: &Project, library: &Library, units: Units, precision: Length) -> CutList {
    let evaluation = evaluate(project, library);
    let show = |length: Length| length.show(units, precision).text;
    let label = |part: &Part| {
        project
            .products
            .iter()
            .find(|p| p.id == part.product)
            .map_or_else(String::new, |p| p.label.clone())
    };
    let material = |part: &Part| {
        library
            .materials
            .iter()
            .find(|m| m.id == part.material)
            .map_or_else(String::new, |m| m.name.clone())
    };
    let rows = evaluation
        .parts
        .iter()
        .map(|part| {
            let banded = part.edges.iter().filter(|e| e.is_some()).count();
            CutRow {
                product: label(part),
                part: part_name(part),
                material: material(part),
                quantity: part.quantity.to_string(),
                length: show(part.length),
                width: show(part.width),
                thickness: show(part.thickness),
                cut_length: show(part.cut_length),
                cut_width: show(part.cut_width),
                edges: match banded {
                    0 => "none".into(),
                    1 => "1 edge".into(),
                    n => format!("{n} edges"),
                },
            }
        })
        .collect();
    let problems = evaluation
        .problems
        .iter()
        .map(|p| {
            let kind = if p.blocking { "not evaluated" } else { "note" };
            format!("{}, {kind}: {}", p.product, p.message)
        })
        .collect();
    CutList { rows, problems }
}

/// A name for people: "Left side", "Front stretcher", "Door 2".
fn part_name(part: &Part) -> String {
    let last = part.key.rsplit('/').next().unwrap_or_default();
    match (part.role, last) {
        ("side", "side.left") => "Left side".into(),
        ("side", "side.right") => "Right side".into(),
        ("stretcher", "stretcher.front") => "Front stretcher".into(),
        ("stretcher", "stretcher.back") => "Back stretcher".into(),
        ("bottom", _) => "Bottom".into(),
        ("top", _) => "Top".into(),
        ("back", _) => "Back".into(),
        ("door", door) => format!("Door {}", door.trim_start_matches("door.")),
        ("drawerFront", _) => "Drawer front".into(),
        ("falseFront", _) => "False front".into(),
        ("drawerSide", "box.side.left") => "Drawer box, left side".into(),
        ("drawerSide", "box.side.right") => "Drawer box, right side".into(),
        ("drawerSubFront", _) => "Drawer box, front".into(),
        ("drawerBack", _) => "Drawer box, back".into(),
        ("drawerBottom", _) => "Drawer box, bottom".into(),
        ("shelf", shelf) => format!("Shelf {}", shelf.trim_start_matches("shelf.")),
        (role, _) => role.into(),
    }
}
