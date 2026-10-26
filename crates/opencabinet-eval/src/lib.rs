// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The evaluator: turns products into parts by the rules of their
//! construction standard (RFC-0001 §9).
//!
//! Evaluation is a pure function of the authored data: the same project
//! always gives the same parts, to the nanometer, on every computer.
//!
//! # What this version evaluates
//!
//! Frameless base cabinets (`base-cabinet@1`) with full-overlay slab doors,
//! drawer fronts and false fronts. Notation: `W`, `H`, `D` the product's
//! width, height and depth; `T` the actual thickness of the `case`
//! material; `K` the toe kick height; `S` the stretcher width; `G` the
//! front gap; `g` the back groove depth. The box height is `Hb = H − K`.
//!
//! | Part | Length × width | Banded edges |
//! | --- | --- | --- |
//! | `case/side.left`, `case/side.right` | `Hb` × `D` | front (`y1`) |
//! | `case/bottom` | `W − 2T` × `D` | front (`y1`) |
//! | `case/stretcher.front` | `W − 2T` × `S` | front (`y1`) |
//! | `case/stretcher.back` | `W − 2T` × `S` | none |
//! | `case/back`, grooved | `Hb − T + g` × `W − 2T + 2g` | none |
//! | `case/back`, applied | `Hb` × `W` | none |
//!
//! Fronts cover the box face, `W` × `Hb`, divided by the front tree. Each
//! front is its opening less `G`: half on each side, so neighbors are `G`
//! apart. A pair of doors splits its opening in two first. Fronts are
//! banded on all four edges; doors run their grain up, drawer and false
//! fronts across.
//!
//! The cut size of a part is its finished size less the thickness of the
//! bands on its ends (length) and sides (width).
//!
//! Anything else, such as other product types, face frames, five-piece
//! doors, drawer boxes, shelves and overrides, makes the product an error
//! rather than giving parts that would be wrong.

pub mod front;
pub mod rules;

use std::cell::RefCell;
use std::collections::HashMap;

use opencabinet_model::{
    EdgeBand, FrontStyle, Id, InsertKind, Length, Library, Method, Product, Project, SheetMaterial,
};

use crate::front::{Opening, Rect, openings};
use crate::rules::{Back, Overlay, Rules};

/// The four edges of a part: `x0` and `x1` are the ends of its length,
/// `y0` and `y1` the sides of its width.
pub type Edges = [Option<Id>; 4];

const Y1: usize = 3;

/// A part of a product, with everything a cut list needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// The product it belongs to.
    pub product: Id,
    /// A key that identifies the part across evaluations, such as
    /// `case/side.left` or `front/<node id>/door.1`.
    pub key: String,
    /// What the part is, such as `side` or `door`.
    pub role: &'static str,
    /// The sheet material it is cut from.
    pub material: Id,
    /// Along the grain.
    pub length: Length,
    /// Across the grain.
    pub width: Length,
    /// The material's actual thickness.
    pub thickness: Length,
    /// The length to cut, before banding.
    pub cut_length: Length,
    /// The width to cut, before banding.
    pub cut_width: Length,
    /// The edge band on each edge, if any.
    pub edges: Edges,
    /// How many of this part the job needs.
    pub quantity: u32,
}

/// Something about a product that the user should know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The product, by its label.
    pub product: String,
    /// What is wrong.
    pub message: String,
    /// Whether the product could not be evaluated at all.
    pub blocking: bool,
}

/// The result of evaluating a project.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Evaluation {
    /// Every part of every product that could be evaluated, in product
    /// order.
    pub parts: Vec<Part>,
    /// Products that could not be evaluated, and things worth knowing.
    pub problems: Vec<Problem>,
}

/// Evaluates every product of a project.
#[must_use]
pub fn evaluate(project: &Project, library: &Library) -> Evaluation {
    let mut placements: HashMap<Id, u32> = HashMap::new();
    for placement in project.rooms.iter().flat_map(|r| &r.placements) {
        *placements.entry(placement.product).or_default() += 1;
    }
    let mut evaluation = Evaluation::default();
    for product in &project.products {
        // Placed products count their placements; others their quantity.
        let quantity = placements
            .get(&product.id)
            .copied()
            .unwrap_or_else(|| product.quantity.unwrap_or(1));
        let problem = |message, blocking| Problem {
            product: product.label.clone(),
            message,
            blocking,
        };
        match evaluate_product(product, library, quantity) {
            Ok(evaluated) => {
                evaluation.parts.extend(evaluated.parts);
                let notes = evaluated.notes.into_iter().map(|n| problem(n, false));
                evaluation.problems.extend(notes);
            }
            Err(message) => evaluation.problems.push(problem(message, true)),
        }
    }
    evaluation
}

/// The parts of one product, and what is worth knowing about them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Evaluated {
    /// The product's parts.
    pub parts: Vec<Part>,
    /// Things the user should know that did not stop the evaluation, such
    /// as edges left bare.
    pub notes: Vec<String>,
}

/// Evaluates one product, of which the job needs `quantity`.
///
/// # Errors
/// A message for the user when the product cannot be evaluated.
pub fn evaluate_product(
    product: &Product,
    library: &Library,
    quantity: u32,
) -> Result<Evaluated, String> {
    if product.product_type != "base-cabinet@1" {
        return Err(format!(
            "product type `{}` cannot be evaluated yet",
            product.product_type
        ));
    }
    if !product.overrides.is_empty() {
        return Err("overrides cannot be applied yet".into());
    }
    let standard = library
        .standards
        .iter()
        .find(|s| s.id == product.standard)
        .ok_or("the standard is not in the library")?;
    if standard.method != Method::Frameless {
        return Err("face frame cabinets cannot be evaluated yet".into());
    }
    let rules = Rules::of(standard)?;
    if rules.fronts.overlay != Overlay::Full {
        return Err("only full overlay fronts can be evaluated yet".into());
    }

    let lookup = Lookup {
        product,
        library,
        notes: RefCell::new(Vec::new()),
    };
    let case = lookup.material("case")?;
    let back = lookup.material("back")?;
    let case_band = lookup.band("edgeBandCase");
    let front_band = lookup.band("edgeBandFronts");

    let (w, h, d) = (product.width, product.height, product.depth);
    let t = case.actual_thickness;
    let box_height = h - rules.toe_kick.height;
    let inner_width = w - t * 2;
    let stretcher = rules.case.stretcher_width;
    if box_height <= Length::ZERO || inner_width <= Length::ZERO {
        return Err("the cabinet is too small for its toe kick and sides".into());
    }

    let maker = PartMaker { product, quantity };
    let front_edge = banded(case_band, &[Y1]);
    let mut parts = vec![
        maker.part("case/side.left", "side", case, box_height, d, front_edge),
        maker.part("case/side.right", "side", case, box_height, d, front_edge),
        maker.part("case/bottom", "bottom", case, inner_width, d, front_edge),
        maker.part(
            "case/stretcher.front",
            "stretcher",
            case,
            inner_width,
            stretcher,
            front_edge,
        ),
        maker.part(
            "case/stretcher.back",
            "stretcher",
            case,
            inner_width,
            stretcher,
            BARE,
        ),
    ];
    let (back_length, back_width) = match rules.back {
        Back::Grooved { groove_depth } => (
            box_height - t + groove_depth,
            inner_width + groove_depth * 2,
        ),
        Back::Applied => (box_height, w),
    };
    parts.push(maker.part("case/back", "back", back, back_length, back_width, BARE));

    if let Some(root) = &product.front {
        let face = Rect {
            x: Length::ZERO,
            y: rules.toe_kick.height,
            width: w,
            height: box_height,
        };
        let fronts = banded(front_band, &[0, 1, 2, 3]);
        for opening in openings(root, face)? {
            fronts_of(
                &lookup,
                &maker,
                &opening,
                rules.fronts.gap,
                fronts,
                &mut parts,
            )?;
        }
    }
    Ok(Evaluated {
        parts,
        notes: lookup.notes.into_inner(),
    })
}

/// The fronts an opening's insert produces.
fn fronts_of(
    lookup: &Lookup<'_>,
    maker: &PartMaker<'_>,
    opening: &Opening<'_>,
    gap: Length,
    edges: Bands<'_>,
    parts: &mut Vec<Part>,
) -> Result<(), String> {
    let insert = opening.insert;
    if matches!(
        insert.style,
        Some(FrontStyle::FivePiece | FrontStyle::Other(_))
    ) {
        return Err("only slab fronts can be evaluated yet".into());
    }
    let prefix = format!("front/{}", opening.node.id);
    match &insert.kind {
        InsertKind::Doors => {
            let material = lookup.material("doors")?;
            let count = u32::from(insert.count.unwrap_or(1));
            if !(1..=2).contains(&count) {
                return Err("an opening takes one or two doors".into());
            }
            let mut x = opening.rect.x;
            for (i, width) in opening.rect.width.split(count).into_iter().enumerate() {
                let door = Rect {
                    x,
                    width,
                    ..opening.rect
                }
                .inset(gap);
                x += width;
                let key = format!("{prefix}/door.{}", i + 1);
                // Doors run their grain up.
                parts.push(maker.part(&key, "door", material, door.height, door.width, edges));
            }
        }
        InsertKind::Drawer | InsertKind::FalseFront => {
            let material = lookup.material_or("drawerFronts", "doors")?;
            let front = opening.rect.inset(gap);
            let (key, role) = if insert.kind == InsertKind::Drawer {
                (format!("{prefix}/drawer/front"), "drawerFront")
            } else {
                (format!("{prefix}/falseFront"), "falseFront")
            };
            // Drawer and false fronts run their grain across.
            parts.push(maker.part(&key, role, material, front.width, front.height, edges));
            if insert.kind == InsertKind::Drawer {
                lookup.note("drawer boxes are not evaluated yet");
            }
        }
        InsertKind::Open => {
            if insert.shelves.unwrap_or(0) > 0 {
                return Err("shelves cannot be evaluated yet".into());
            }
        }
        InsertKind::Appliance | InsertKind::None => {}
        InsertKind::Other(kind) => {
            return Err(format!(
                "unknown insert `{kind}`; open with a newer OpenCabinet"
            ));
        }
    }
    Ok(())
}

/// Finds the library items a product's material roles point to.
struct Lookup<'p> {
    product: &'p Product,
    library: &'p Library,
    notes: RefCell<Vec<String>>,
}

impl<'p> Lookup<'p> {
    fn material(&self, role: &str) -> Result<&'p SheetMaterial, String> {
        let id = self
            .product
            .materials
            .get(role)
            .ok_or_else(|| format!("no material chosen for `{role}`"))?;
        self.library
            .materials
            .iter()
            .find(|m| m.id == *id)
            .ok_or_else(|| format!("the material for `{role}` is not in the library"))
    }

    /// The material for `role`, or for `fallback` when `role` has none.
    fn material_or(&self, role: &str, fallback: &str) -> Result<&'p SheetMaterial, String> {
        if self.product.materials.contains_key(role) {
            self.material(role)
        } else {
            self.material(fallback)
        }
    }

    /// The edge band for `role`; without one, edges stay unbanded and the
    /// user is told.
    fn band(&self, role: &str) -> Option<&'p EdgeBand> {
        let band = self
            .product
            .materials
            .get(role)
            .and_then(|id| self.library.edge_bands.iter().find(|b| b.id == *id));
        if band.is_none() {
            self.note(&format!(
                "no edge band chosen for `{role}`; those edges are left bare"
            ));
        }
        band
    }

    /// Records a note once, however many parts it concerns.
    fn note(&self, message: &str) {
        let mut notes = self.notes.borrow_mut();
        if !notes.iter().any(|n| n == message) {
            notes.push(message.to_owned());
        }
    }
}

/// The bands on a part's four edges, while it is being built.
type Bands<'p> = [Option<&'p EdgeBand>; 4];

const BARE: Bands<'static> = [None; 4];

/// `band` on the given edges, the others bare.
fn banded<'p>(band: Option<&'p EdgeBand>, edges: &[usize]) -> Bands<'p> {
    let mut out = BARE;
    for &edge in edges {
        out[edge] = band;
    }
    out
}

/// Builds parts for one product, with its quantity.
struct PartMaker<'p> {
    product: &'p Product,
    quantity: u32,
}

impl PartMaker<'_> {
    fn part(
        &self,
        key: &str,
        role: &'static str,
        material: &SheetMaterial,
        length: Length,
        width: Length,
        bands: Bands<'_>,
    ) -> Part {
        let thickness = |edge: usize| bands[edge].map_or(Length::ZERO, |b| b.thickness);
        Part {
            product: self.product.id,
            key: key.into(),
            role,
            material: material.id,
            length,
            width,
            thickness: material.actual_thickness,
            cut_length: length - thickness(0) - thickness(1),
            cut_width: width - thickness(2) - thickness(3),
            edges: bands.map(|b| b.map(|b| b.id)),
            quantity: self.quantity,
        }
    }
}
