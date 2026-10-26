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
//! Frameless base, wall and tall cabinets (`base-cabinet@1`,
//! `wall-cabinet@1`, `tall-cabinet@1`) with full-overlay slab doors, drawer
//! fronts and false fronts, drawer boxes and adjustable shelves. Notation:
//! `W`, `H`, `D` the product's width, height and depth; `T` the actual
//! thickness of the `case` material; `K` the toe kick height, none for a
//! wall cabinet; `S` the stretcher width; `G` the front gap; `g` the back
//! groove depth. The box height is `Hb = H − K`.
//!
//! | Part | Length × width | Banded edges |
//! | --- | --- | --- |
//! | `case/side.left`, `case/side.right` | `Hb` × `D` | front (`y1`) |
//! | `case/bottom` | `W − 2T` × `D` | front (`y1`) |
//! | `case/stretcher.front`, base only | `W − 2T` × `S` | front (`y1`) |
//! | `case/stretcher.back`, base only | `W − 2T` × `S` | none |
//! | `case/top`, wall and tall | `W − 2T` × `D` | front (`y1`) |
//! | `case/back`, grooved, base | `Hb − T + g` × `W − 2T + 2g` | none |
//! | `case/back`, grooved, wall and tall | `Hb − 2T + 2g` × `W − 2T + 2g` | none |
//! | `case/back`, applied | `Hb` × `W` | none |
//!
//! Fronts cover the box face, `W` × `Hb`, divided by the front tree. Each
//! front is its opening less `G`: half on each side, so neighbors are `G`
//! apart. A pair of doors splits its opening in two first. Fronts are
//! banded on all four edges; doors run their grain up, drawer and false
//! fronts across.
//!
//! A **drawer box** (from the standard's `drawers` rules, the `drawerBox`
//! and `drawerBottom` materials, `t` their thickness) is as deep as the
//! longest stocked slide `L` that fits in `D` less the rear clearance; as
//! high as its opening less the vertical clearance `v`; and as wide as the
//! inside less the slide clearance `c` on each side. The sides run the full
//! depth and hold the front and back; the bottom sits in grooves `b` deep:
//!
//! | Part | Length × width |
//! | --- | --- |
//! | `…/drawer/box.side.left`, `.right` | `L` × `opening − v` |
//! | `…/drawer/box.front`, `.back` | `W − 2T − 2c − 2t` × `opening − v` |
//! | `…/drawer/box.bottom` | `W − 2T − 2c − 2t + 2b` × `L − 2t + 2b` |
//!
//! **Shelves** of an open section or behind doors (the `shelves` rules,
//! the `shelves` material, or the case's) span the inside: `W − 2T` less
//! the side clearance, by `D` less the front setback and the back
//! allowance, banded in front like the case.
//!
//! The cut size of a part is its finished size less the thickness of the
//! bands on its ends (length) and sides (width).
//!
//! Anything else, such as other product types, face frames, five-piece
//! doors and overrides, makes the product an error rather than giving parts
//! that would be wrong.

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
    let kind = Kind::of(&product.product_type).ok_or_else(|| {
        format!(
            "product type `{}` cannot be evaluated yet",
            product.product_type
        )
    })?;
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
    // Wall cabinets hang; base and tall ones stand on a toe kick.
    let toe_kick = match kind {
        Kind::Wall => Length::ZERO,
        Kind::Base | Kind::Tall => rules.toe_kick.height,
    };
    let box_height = h - toe_kick;
    let inner_width = w - t * 2;
    if box_height <= t * 2 || inner_width <= Length::ZERO {
        return Err("the cabinet is too small for its toe kick and sides".into());
    }

    let maker = PartMaker { product, quantity };
    let front_edge = banded(case_band, &[Y1]);
    let size = CaseSize {
        box_height,
        inner_width,
        width: w,
        depth: d,
    };
    let mut parts = case_parts(kind, size, &rules, &maker, (case, back), front_edge);

    if let Some(root) = &product.front {
        let face = Rect {
            x: Length::ZERO,
            y: toe_kick,
            width: w,
            height: box_height,
        };
        let cabinet = Cabinet {
            lookup: &lookup,
            maker: &maker,
            rules: &rules,
            inner_width,
            depth: d,
            fronts: banded(front_band, &[0, 1, 2, 3]),
            shelf_front: front_edge,
        };
        for opening in openings(root, face)? {
            cabinet.insert(&opening, &mut parts)?;
        }
    }
    Ok(Evaluated {
        parts,
        notes: lookup.notes.into_inner(),
    })
}

/// The overall sizes a case is built from.
#[derive(Clone, Copy)]
struct CaseSize {
    /// Above the toe kick.
    box_height: Length,
    /// Between the sides.
    inner_width: Length,
    width: Length,
    depth: Length,
}

/// The case's parts: sides, bottom, top or stretchers, and back.
fn case_parts(
    kind: Kind,
    size: CaseSize,
    rules: &Rules,
    maker: &PartMaker<'_>,
    (case, back): (&SheetMaterial, &SheetMaterial),
    front_edge: Bands<'_>,
) -> Vec<Part> {
    let CaseSize {
        box_height,
        inner_width,
        width: w,
        depth: d,
    } = size;
    let t = case.actual_thickness;
    let mut parts = vec![
        maker.part("case/side.left", "side", case, box_height, d, front_edge),
        maker.part("case/side.right", "side", case, box_height, d, front_edge),
        maker.part("case/bottom", "bottom", case, inner_width, d, front_edge),
    ];
    // A base cabinet is tied at the top by two stretchers, so its back is
    // grooved into the bottom only; wall and tall cabinets have a full top,
    // grooved like the bottom.
    let grooved_ends = match kind {
        Kind::Base => {
            let stretcher = rules.case.stretcher_width;
            parts.extend([
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
            ]);
            1
        }
        Kind::Wall | Kind::Tall => {
            parts.push(maker.part("case/top", "top", case, inner_width, d, front_edge));
            2
        }
    };
    let (back_length, back_width) = match rules.back {
        Back::Grooved { groove_depth } => (
            box_height - (t - groove_depth) * grooved_ends,
            inner_width + groove_depth * 2,
        ),
        Back::Applied => (box_height, w),
    };
    parts.push(maker.part("case/back", "back", back, back_length, back_width, BARE));

    parts
}

/// The kinds of cabinet the evaluator builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Stands on a toe kick, tied at the top by stretchers.
    Base,
    /// Hangs on the wall, with a full top and no toe kick.
    Wall,
    /// Stands on a toe kick, with a full top.
    Tall,
}

impl Kind {
    fn of(product_type: &str) -> Option<Self> {
        match product_type {
            "base-cabinet@1" => Some(Self::Base),
            "wall-cabinet@1" => Some(Self::Wall),
            "tall-cabinet@1" => Some(Self::Tall),
            _ => None,
        }
    }
}

/// What the parts of a front's inserts depend on.
struct Cabinet<'c, 'p> {
    lookup: &'c Lookup<'p>,
    maker: &'c PartMaker<'p>,
    rules: &'c Rules,
    /// Between the sides.
    inner_width: Length,
    depth: Length,
    /// The bands on a front's four edges.
    fronts: Bands<'p>,
    /// The band on a shelf's front edge.
    shelf_front: Bands<'p>,
}

impl Cabinet<'_, '_> {
    /// The parts an opening's insert produces.
    fn insert(&self, opening: &Opening<'_>, parts: &mut Vec<Part>) -> Result<(), String> {
        let insert = opening.insert;
        if matches!(
            insert.style,
            Some(FrontStyle::FivePiece | FrontStyle::Other(_))
        ) {
            return Err("only slab fronts can be evaluated yet".into());
        }
        let prefix = format!("front/{}", opening.node.id);
        let gap = self.rules.fronts.gap;
        match &insert.kind {
            InsertKind::Doors => {
                let material = self.lookup.material("doors")?;
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
                    let (length, width) = (door.height, door.width);
                    parts.push(
                        self.maker
                            .part(&key, "door", material, length, width, self.fronts),
                    );
                }
                self.shelves(&prefix, insert.shelves, parts)?;
            }
            InsertKind::Drawer | InsertKind::FalseFront => {
                let material = self.lookup.material_or("drawerFronts", "doors")?;
                let front = opening.rect.inset(gap);
                let (key, role) = if insert.kind == InsertKind::Drawer {
                    (format!("{prefix}/drawer/front"), "drawerFront")
                } else {
                    (format!("{prefix}/falseFront"), "falseFront")
                };
                // Drawer and false fronts run their grain across.
                let (length, width) = (front.width, front.height);
                parts.push(
                    self.maker
                        .part(&key, role, material, length, width, self.fronts),
                );
                if insert.kind == InsertKind::Drawer {
                    self.drawer_box(&prefix, opening.rect.height, parts)?;
                }
            }
            InsertKind::Open => self.shelves(&prefix, insert.shelves, parts)?,
            InsertKind::Appliance | InsertKind::None => {}
            InsertKind::Other(kind) => {
                return Err(format!(
                    "unknown insert `{kind}`; open with a newer OpenCabinet"
                ));
            }
        }
        Ok(())
    }

    /// The box of a drawer whose opening is `opening_height` tall: sides
    /// the full depth, front and back between them, and the bottom in a
    /// groove in all four.
    fn drawer_box(
        &self,
        prefix: &str,
        opening_height: Length,
        parts: &mut Vec<Part>,
    ) -> Result<(), String> {
        let rules = self
            .rules
            .drawers
            .as_ref()
            .ok_or("the standard has no `drawers` rules")?;
        let sides = self.lookup.material("drawerBox")?;
        let bottom = self.lookup.material("drawerBottom")?;
        let t = sides.actual_thickness;
        let room = self.depth - rules.rear_clearance;
        let depth = rules
            .slide_lengths
            .iter()
            .copied()
            .filter(|&l| l <= room)
            .max()
            .ok_or("no slide in the standard is short enough for this cabinet")?;
        let height = opening_height - rules.vertical_clearance;
        let width = self.inner_width - rules.side_clearance * 2;
        let between = width - t * 2;
        if height <= Length::ZERO || between <= Length::ZERO {
            return Err("the drawer opening is too small for a drawer box".into());
        }
        let g = rules.bottom_groove_depth;
        let part = |name: &str, role, material, length, width| {
            let key = format!("{prefix}/drawer/box.{name}");
            self.maker.part(&key, role, material, length, width, BARE)
        };
        parts.extend([
            part("side.left", "drawerSide", sides, depth, height),
            part("side.right", "drawerSide", sides, depth, height),
            part("front", "drawerSubFront", sides, between, height),
            part("back", "drawerBack", sides, between, height),
            part(
                "bottom",
                "drawerBottom",
                bottom,
                between + g * 2,
                depth - t * 2 + g * 2,
            ),
        ]);
        Ok(())
    }

    /// `count` adjustable shelves across the cabinet's inside.
    fn shelves(
        &self,
        prefix: &str,
        count: Option<u8>,
        parts: &mut Vec<Part>,
    ) -> Result<(), String> {
        let count = count.unwrap_or(0);
        if count == 0 {
            return Ok(());
        }
        let rules = self
            .rules
            .shelves
            .as_ref()
            .ok_or("the standard has no `shelves` rules")?;
        let material = self.lookup.material_or("shelves", "case")?;
        let length = self.inner_width - rules.side_clearance;
        let width = self.depth - rules.front_setback - rules.back_allowance;
        if length <= Length::ZERO || width <= Length::ZERO {
            return Err("the cabinet is too small for its shelves".into());
        }
        for i in 1..=count {
            let key = format!("{prefix}/shelf.{i}");
            let part = self
                .maker
                .part(&key, "shelf", material, length, width, self.shelf_front);
            parts.push(part);
        }
        Ok(())
    }
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
