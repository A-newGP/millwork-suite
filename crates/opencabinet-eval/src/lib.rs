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
//! Frameless and face frame base, wall and tall cabinets (`base-cabinet@1`,
//! `wall-cabinet@1`, `tall-cabinet@1`) with slab or five-piece doors,
//! drawer fronts and false fronts, drawer boxes and adjustable shelves:
//! full overlay when frameless, partial overlay or inset with a face frame.
//! Notation:
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
//! A **five-piece** front (the standard's `fivePiece` rules; the
//! `frontFrames` and `frontPanels` materials, or the doors') is two stiles
//! `s` wide the full height of the front, two rails `r` wide between them
//! that reach `j` into each stile, and a panel in grooves `p` deep, `c`
//! smaller than its grooves so it can move. For a front `F` wide and `E`
//! high: rails are `F − 2s + 2j` long, the panel `E − 2r + 2p − c` by
//! `F − 2s + 2p − c`. Frames are not banded.
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
//! A **face frame** (the standard's `faceFrame` rules, the `faceFrame`
//! material, `F` its thickness) covers the box face, `W` × `Hb`, with
//! butt joints: two stiles `s` wide the full height, top and bottom rails
//! `rt` and `rb` wide between them, and for each split of the front a rail
//! (`r` wide) or a mullion (`m` wide) between its openings, as long as the
//! opening it divides. The frame is part of the depth and stands out past
//! each side of the case by the overhang `o`, so the case is `W − 2o` wide
//! and `D − F` deep, and its front edges are not banded. The top of the
//! bottom is flush with the top of the bottom rail: the bottom is raised
//! `rb − T`, which shortens a grooved back by as much. Every frameless
//! formula above holds with these widths and depth.
//!
//! The front tree divides the frame's opening, `W − 2s` × `Hb − rt − rb`,
//! less its rails and mullions; a fixed size in the tree is the size of an
//! opening. A partial overlay front is its opening plus the overlay amount
//! on each edge; an inset front is its opening less `G` on each edge. The
//! doors of a pair are `G` apart. Overlays must stay on the frame and
//! clear each other by `G` over a rail or mullion. A drawer box is as wide
//! as its frame opening less the slide clearances, rather than the inside
//! of the case.
//!
//! The cut size of a part is its finished size less the thickness of the
//! bands on its ends (length) and sides (width).
//!
//! Anything else, such as other product types, other face frame joinery
//! and overrides,
//! makes the product an error rather than giving parts that would be wrong.

pub mod front;
pub mod rules;

use std::cell::RefCell;
use std::collections::HashMap;

use opencabinet_model::{
    EdgeBand, FrontStyle, Id, InsertKind, Length, Library, Method, Product, Project, SheetMaterial,
    SplitDirection,
};

use crate::front::{Divider, Dividers, Layout, Opening, Rect, lay_out};
use crate::rules::{Back, FaceFrame, Fronts, Overlay, Rules};

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
    let rules = Rules::of(standard)?;
    let face_frame = face_frame(standard.method, &rules)?;

    let lookup = Lookup {
        product,
        library,
        notes: RefCell::new(Vec::new()),
    };
    let case = lookup.material("case")?;
    let back = lookup.material("back")?;
    let frame_material = match face_frame {
        Some(_) => Some(lookup.material("faceFrame")?),
        None => None,
    };
    // A face frame covers the front edges of the case.
    let case_band = match face_frame {
        Some(_) => None,
        None => lookup.band("edgeBandCase"),
    };

    let (w, h, d) = (product.width, product.height, product.depth);
    let t = case.actual_thickness;
    // Wall cabinets hang; base and tall ones stand on a toe kick.
    let toe_kick = match kind {
        Kind::Wall => Length::ZERO,
        Kind::Base | Kind::Tall => rules.toe_kick.height,
    };
    let box_height = h - toe_kick;
    // The frame stands out past the case's sides and is part of its depth.
    let overhang = face_frame.map_or(Length::ZERO, |f| f.overhang);
    let frame_thickness = frame_material.map_or(Length::ZERO, |m| m.actual_thickness);
    let case_width = w - overhang * 2;
    let case_depth = d - frame_thickness;
    let inner_width = case_width - t * 2;
    if box_height <= t * 2 || inner_width <= Length::ZERO || case_depth <= Length::ZERO {
        return Err("the cabinet is too small for its toe kick and sides".into());
    }
    // The top of the bottom is flush with the top of the frame's bottom rail.
    let bottom_raise = face_frame.map_or(Length::ZERO, |f| f.bottom_rail() - t);
    if bottom_raise < Length::ZERO {
        return Err(
            "the face frame's bottom rail is narrower than the case bottom is thick".into(),
        );
    }

    let maker = PartMaker { product, quantity };
    let front_edge = banded(case_band, &[Y1]);
    let size = CaseSize {
        box_height,
        inner_width,
        width: case_width,
        depth: case_depth,
        bottom_raise,
    };
    let mut parts = case_parts(kind, size, &rules, &maker, (case, back), front_edge);

    let face = Rect {
        x: Length::ZERO,
        y: toe_kick,
        width: w,
        height: box_height,
    };
    let (layout, fit) = lay_out_front(product, face_frame, face, &rules.fronts)?;
    if let (Some(frame), Some(material)) = (face_frame, frame_material) {
        fit.check_frame(frame, &layout.dividers)?;
        parts.extend(frame_parts(frame, material, face, &layout.dividers, &maker));
    }
    let cabinet = Cabinet {
        lookup: &lookup,
        maker: &maker,
        rules: &rules,
        fit,
        inner_width,
        depth: case_depth,
    };
    for opening in &layout.openings {
        cabinet.insert(opening, &mut parts)?;
    }
    Ok(Evaluated {
        parts,
        notes: lookup.notes.into_inner(),
    })
}

/// The face frame rules of a standard built by `method`, which has none
/// when it is frameless.
fn face_frame(method: Method, rules: &Rules) -> Result<Option<&FaceFrame>, String> {
    match method {
        Method::Frameless => {
            if rules.fronts.overlay != Overlay::Full {
                return Err("frameless cabinets take full overlay fronts".into());
            }
            Ok(None)
        }
        Method::FaceFrame => rules
            .face_frame
            .as_ref()
            .map(Some)
            .ok_or_else(|| "the standard has no `faceFrame` rules".into()),
    }
}

/// The product's front laid over `face`, inside the face frame if there is
/// one, and how its fronts fit.
fn lay_out_front<'p>(
    product: &'p Product,
    face_frame: Option<&FaceFrame>,
    face: Rect,
    fronts: &Fronts,
) -> Result<(Layout<'p>, Fit), String> {
    let (opening, dividers, fit) = match face_frame {
        None => (
            face,
            Dividers::default(),
            Fit::Frameless { gap: fronts.gap },
        ),
        Some(frame) => (frame.opening(face)?, frame.dividers(), Fit::framed(fronts)?),
    };
    let layout = match &product.front {
        Some(root) => lay_out(root, opening, dividers)?,
        None => Layout::default(),
    };
    Ok((layout, fit))
}

/// The members of a face frame the size of `face`: stiles the full height,
/// the top and bottom rails between them, and the rails and mullions
/// between openings. None of them is banded.
fn frame_parts(
    frame: &FaceFrame,
    material: &SheetMaterial,
    face: Rect,
    dividers: &[Divider<'_>],
    maker: &PartMaker<'_>,
) -> Vec<Part> {
    let stile = frame.stile_width;
    let between = face.width - stile * 2;
    let part =
        |key: &str, role, length, width| maker.part(key, role, material, length, width, BARE);
    // Every member runs its grain along its length.
    let mut parts = vec![
        part("frame/stile.left", "frameStile", face.height, stile),
        part("frame/stile.right", "frameStile", face.height, stile),
        part("frame/rail.top", "frameRail", between, frame.top_rail()),
        part(
            "frame/rail.bottom",
            "frameRail",
            between,
            frame.bottom_rail(),
        ),
    ];
    for divider in dividers {
        let r = divider.rect;
        let (name, role, length, width) = match divider.direction {
            SplitDirection::Horizontal => ("rail", "frameRail", r.width, r.height),
            SplitDirection::Vertical => ("mullion", "frameMullion", r.height, r.width),
        };
        let key = format!("frame/{}/{name}.{}", divider.split.id, divider.index);
        parts.push(part(&key, role, length, width));
    }
    parts
}

/// What the evaluator works out from a standard's face frame rules.
trait FrameSizes {
    fn top_rail(&self) -> Length;
    fn bottom_rail(&self) -> Length;
    /// The widths of the rails and mullions between openings.
    fn dividers(&self) -> Dividers;
    /// The opening inside the stiles and the top and bottom rails of a
    /// frame the size of `face`.
    fn opening(&self, face: Rect) -> Result<Rect, String>;
}

impl FrameSizes for FaceFrame {
    fn top_rail(&self) -> Length {
        self.top_rail_width.unwrap_or(self.rail_width)
    }

    fn bottom_rail(&self) -> Length {
        self.bottom_rail_width.unwrap_or(self.rail_width)
    }

    fn dividers(&self) -> Dividers {
        Dividers {
            rail: self.rail_width,
            mullion: self.mullion_width.unwrap_or(self.stile_width),
        }
    }

    fn opening(&self, face: Rect) -> Result<Rect, String> {
        let opening = Rect {
            x: face.x + self.stile_width,
            y: face.y + self.bottom_rail(),
            width: face.width - self.stile_width * 2,
            height: face.height - self.top_rail() - self.bottom_rail(),
        };
        if opening.width <= Length::ZERO || opening.height <= Length::ZERO {
            return Err("the cabinet is too small for its face frame".into());
        }
        Ok(opening)
    }
}

/// How fronts sit on their openings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fit {
    /// Frameless: fronts share the box face, `gap` apart.
    Frameless { gap: Length },
    /// Face frame: fronts cover the frame by `by` around their opening, or
    /// stand back from it when `by` is less than zero (inset); the doors of
    /// a pair are `gap` apart.
    Framed { by: Length, gap: Length },
}

impl Fit {
    /// How the fronts of a face frame cabinet fit.
    fn framed(fronts: &Fronts) -> Result<Self, String> {
        let by = match fronts.overlay {
            Overlay::Partial => fronts
                .overlay_amount
                .ok_or("partial overlay fronts need an `overlayAmount`")?,
            Overlay::Inset => -fronts.gap,
            Overlay::Full => {
                return Err("face frame cabinets take partial overlay or inset fronts".into());
            }
        };
        Ok(Self::Framed {
            by,
            gap: fronts.gap,
        })
    }

    /// That overlaid fronts stay on the frame, and clear each other by at
    /// least the gap over the rails and mullions between them.
    fn check_frame(self, frame: &FaceFrame, dividers: &[Divider<'_>]) -> Result<(), String> {
        let Self::Framed { by, gap } = self else {
            return Ok(());
        };
        if by <= Length::ZERO {
            return Ok(());
        }
        let edge = frame
            .stile_width
            .min(frame.top_rail())
            .min(frame.bottom_rail());
        if by > edge {
            return Err("the fronts overlay more than the face frame around them is wide".into());
        }
        for divider in dividers {
            let width = match divider.direction {
                SplitDirection::Horizontal => divider.rect.height,
                SplitDirection::Vertical => divider.rect.width,
            };
            if width - by * 2 < gap {
                return Err(
                    "a rail or mullion is too narrow for the fronts on each side \
                    of it to clear each other by the gap"
                        .into(),
                );
            }
        }
        Ok(())
    }

    /// `count` fronts side by side for `opening`, left to right.
    fn fronts(self, opening: Rect, count: u32) -> Result<Vec<Rect>, String> {
        let mut x = opening.x;
        let fronts: Vec<Rect> = match self {
            // Each front is its share of the face less the gap.
            Self::Frameless { gap } => opening
                .width
                .split(count)
                .into_iter()
                .map(|width| {
                    let front = Rect {
                        x,
                        width,
                        ..opening
                    }
                    .inset(gap);
                    x += width;
                    front
                })
                .collect(),
            // The fronts share what covers the opening, less the gaps
            // between them.
            Self::Framed { by, gap } => {
                let whole = Rect {
                    x: opening.x - by,
                    y: opening.y - by,
                    width: opening.width + by * 2,
                    height: opening.height + by * 2,
                };
                let room = whole.width - gap * (i64::from(count) - 1);
                if room <= Length::ZERO {
                    return Err("the opening is too small for its fronts".into());
                }
                x = whole.x;
                room.split(count)
                    .into_iter()
                    .map(|width| {
                        let front = Rect { x, width, ..whole };
                        x += width + gap;
                        front
                    })
                    .collect()
            }
        };
        if fronts
            .iter()
            .any(|f| f.width <= Length::ZERO || f.height <= Length::ZERO)
        {
            return Err("the opening is too small for its fronts".into());
        }
        Ok(fronts)
    }
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
    /// How far the bottom sits above the bottom of the sides.
    bottom_raise: Length,
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
        bottom_raise,
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
            box_height - (t - groove_depth) * grooved_ends - bottom_raise,
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
    fit: Fit,
    /// Between the sides.
    inner_width: Length,
    /// Of the case, behind any face frame.
    depth: Length,
}

impl<'p> Cabinet<'_, 'p> {
    /// The bands on a slab front's four edges.
    fn front_bands(&self) -> Bands<'p> {
        banded(self.lookup.band("edgeBandFronts"), &[0, 1, 2, 3])
    }

    /// The parts an opening's insert produces.
    fn insert(&self, opening: &Opening<'_>, parts: &mut Vec<Part>) -> Result<(), String> {
        let insert = opening.insert;
        if let Some(FrontStyle::Other(style)) = &insert.style {
            return Err(format!("unknown front style `{style}`"));
        }
        let five_piece = insert.style == Some(FrontStyle::FivePiece);
        let prefix = format!("front/{}", opening.node.id);
        match &insert.kind {
            InsertKind::Doors => {
                let material = self.lookup.material("doors")?;
                let count = u32::from(insert.count.unwrap_or(1));
                if !(1..=2).contains(&count) {
                    return Err("an opening takes one or two doors".into());
                }
                let doors = self.fit.fronts(opening.rect, count)?;
                for (i, door) in doors.into_iter().enumerate() {
                    let key = format!("{prefix}/door.{}", i + 1);
                    if five_piece {
                        self.five_piece(&key, door, parts)?;
                    } else {
                        // Slab doors run their grain up.
                        let (length, width) = (door.height, door.width);
                        let part = self.maker.part(
                            &key,
                            "door",
                            material,
                            length,
                            width,
                            self.front_bands(),
                        );
                        parts.push(part);
                    }
                }
                self.shelves(&prefix, insert.shelves, parts)?;
            }
            InsertKind::Drawer | InsertKind::FalseFront => {
                let material = self.lookup.material_or("drawerFronts", "doors")?;
                let front = self.fit.fronts(opening.rect, 1)?[0];
                let (key, role) = if insert.kind == InsertKind::Drawer {
                    (format!("{prefix}/drawer/front"), "drawerFront")
                } else {
                    (format!("{prefix}/falseFront"), "falseFront")
                };
                if five_piece {
                    self.five_piece(&key, front, parts)?;
                } else {
                    // Slab drawer and false fronts run their grain across.
                    let (length, width) = (front.width, front.height);
                    let part =
                        self.maker
                            .part(&key, role, material, length, width, self.front_bands());
                    parts.push(part);
                }
                if insert.kind == InsertKind::Drawer {
                    self.drawer_box(&prefix, opening.rect, parts)?;
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

    /// A five-piece front the size of `front`: two stiles the full height,
    /// two rails between them reaching into the stiles by the joint depth,
    /// and a panel that floats in grooves. The frame is not banded.
    fn five_piece(&self, key: &str, front: Rect, parts: &mut Vec<Part>) -> Result<(), String> {
        let rules = self
            .rules
            .five_piece
            .as_ref()
            .ok_or("the standard has no `fivePiece` rules")?;
        let frame = self.lookup.material_or("frontFrames", "doors")?;
        let panel = self.lookup.material_or("frontPanels", "doors")?;
        let (stile, rail) = (rules.stile_width, rules.rail_width);
        let between = front.width - stile * 2;
        let panel_height = front.height - rail * 2;
        if between <= Length::ZERO || panel_height <= Length::ZERO {
            return Err("the front is too small for a five-piece frame".into());
        }
        let g = rules.panel_groove_depth;
        let float = rules.panel_clearance;
        let part = |name: &str, role, material, length, width| {
            self.maker.part(
                &format!("{key}/{name}"),
                role,
                material,
                length,
                width,
                BARE,
            )
        };
        parts.extend([
            // Stiles run their grain up, rails across, the panel up.
            part("stile.left", "stile", frame, front.height, stile),
            part("stile.right", "stile", frame, front.height, stile),
            part(
                "rail.top",
                "rail",
                frame,
                between + rules.joint_depth * 2,
                rail,
            ),
            part(
                "rail.bottom",
                "rail",
                frame,
                between + rules.joint_depth * 2,
                rail,
            ),
            part(
                "panel",
                "panel",
                panel,
                panel_height + g * 2 - float,
                between + g * 2 - float,
            ),
        ]);
        Ok(())
    }

    /// The box of a drawer for `opening`: sides the full depth, front and
    /// back between them, and the bottom in a groove in all four.
    fn drawer_box(&self, prefix: &str, opening: Rect, parts: &mut Vec<Part>) -> Result<(), String> {
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
        let height = opening.height - rules.vertical_clearance;
        // Behind a face frame the box must pass through the frame's
        // opening; the slides are set out to it on blocking.
        let room = match self.fit {
            Fit::Frameless { .. } => self.inner_width,
            Fit::Framed { .. } => {
                self.lookup.note(
                    "drawer slides in face frame cabinets mount on blocking out to the \
                     frame opening, which is not in the cut list yet",
                );
                opening.width
            }
        };
        let width = room - rules.side_clearance * 2;
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
        let front = banded(self.lookup.band("edgeBandCase"), &[Y1]);
        let length = self.inner_width - rules.side_clearance;
        let width = self.depth - rules.front_setback - rules.back_allowance;
        if length <= Length::ZERO || width <= Length::ZERO {
            return Err("the cabinet is too small for its shelves".into());
        }
        for i in 1..=count {
            let key = format!("{prefix}/shelf.{i}");
            let part = self
                .maker
                .part(&key, "shelf", material, length, width, front);
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
