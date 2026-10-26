// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Elevations (RFC-0001 §5): each wall seen from the room, with its openings
//! and the products placed on it, and the checks of where products are
//! placed.
//!
//! An elevation runs `x` along the wall from its start, as the placements'
//! offsets do, and `y` up from the floor. A product on a wall takes the
//! wall from its offset to its offset plus its width, its back on the
//! wall's front face, which is to the left walking from the wall's start
//! to its end.
//!
//! The checks give warnings, not errors: a product placed wrong still has
//! the right parts.

use opencabinet_model::{Id, Length, Library, Product, Project, Wall, WallOpeningKind};

use crate::front::Rect;
use crate::view::{FrontView, front_view};

/// A wall seen from the room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elevation {
    /// The wall.
    pub wall: Id,
    /// Its name, such as "North".
    pub name: String,
    /// From its start to its end.
    pub length: Length,
    /// Floor to ceiling.
    pub height: Length,
    /// Its windows, doors and pass-throughs.
    pub openings: Vec<OpeningView>,
    /// The products placed on it, in the room's order.
    pub cabinets: Vec<PlacedCabinet>,
}

/// An opening in a wall.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningView {
    /// A window, a door or a pass-through.
    pub kind: WallOpeningKind,
    /// Where it is on the wall.
    pub rect: Rect,
}

/// A product placed on a wall.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedCabinet {
    /// The placement.
    pub placement: Id,
    /// The product placed.
    pub product: Id,
    /// The product's label.
    pub label: String,
    /// Where it is on the wall.
    pub rect: Rect,
    /// Its front, in its own coordinates; `None` when it cannot be laid
    /// out, and only its outline is drawn.
    pub view: Option<FrontView>,
}

/// A dimension of an elevation, drawn along a line `at` from the wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    /// From `from` to `to` along the wall, drawn at height `at`.
    Horizontal {
        /// Its start.
        from: Length,
        /// Its end.
        to: Length,
        /// The height of its line.
        at: Length,
    },
    /// From `from` to `to` up the wall, drawn at `at` along it.
    Vertical {
        /// Its bottom.
        from: Length,
        /// Its top.
        to: Length,
        /// Where its line is along the wall.
        at: Length,
    },
}

/// How far the rows of dimensions stand off the wall.
const ROW: Length = Length::from_mm(100);

/// The elevation of `wall`, if a room of the project has it.
#[must_use]
pub fn elevation(project: &Project, library: &Library, wall: Id) -> Option<Elevation> {
    let room = project
        .rooms
        .iter()
        .find(|r| r.walls.iter().any(|w| w.id == wall))?;
    let wall = room.walls.iter().find(|w| w.id == wall)?;
    let openings = wall
        .openings
        .iter()
        .map(|o| OpeningView {
            kind: o.kind.clone(),
            rect: Rect {
                x: o.offset,
                y: o.sill_height,
                width: o.width,
                height: o.height,
            },
        })
        .collect();
    let cabinets = room
        .placements
        .iter()
        .filter(|p| p.wall == Some(wall.id))
        .filter_map(|placement| {
            let product = project
                .products
                .iter()
                .find(|p| p.id == placement.product)?;
            Some(PlacedCabinet {
                placement: placement.id,
                product: product.id,
                label: product.label.clone(),
                rect: Rect {
                    x: placement.offset.unwrap_or_default(),
                    y: placement.elevation.unwrap_or_default(),
                    width: product.width,
                    height: product.height,
                },
                view: front_view(product, library).ok(),
            })
        })
        .collect();
    Some(Elevation {
        wall: wall.id,
        name: wall.name.clone(),
        length: wall_length(wall),
        height: wall.height,
        openings,
        cabinets,
    })
}

impl Elevation {
    /// The dimensions a shop drawing gives: below the floor, the widths of
    /// the products standing on it and the gaps between them, then the
    /// whole wall; above the wall, the same for the products hung on it;
    /// beside the wall's end, the heights of every product's bottom and
    /// top.
    #[must_use]
    pub fn dimensions(&self) -> Vec<Dimension> {
        let mut dimensions = Vec::new();
        let (standing, hung): (Vec<&PlacedCabinet>, Vec<&PlacedCabinet>) =
            self.cabinets.iter().partition(|c| c.rect.y <= Length::ZERO);
        for (row, at) in [(standing, -ROW), (hung, self.height + ROW)] {
            if row.is_empty() {
                continue;
            }
            let mut stops = vec![Length::ZERO, self.length];
            for cabinet in row {
                stops.extend([cabinet.rect.x, cabinet.rect.x + cabinet.rect.width]);
            }
            for (from, to) in chain(stops) {
                dimensions.push(Dimension::Horizontal { from, to, at });
            }
        }
        dimensions.push(Dimension::Horizontal {
            from: Length::ZERO,
            to: self.length,
            at: -ROW * 2,
        });
        let mut levels = vec![Length::ZERO, self.height];
        for cabinet in &self.cabinets {
            levels.extend([cabinet.rect.y, cabinet.rect.y + cabinet.rect.height]);
        }
        let at = self.length + ROW;
        for (from, to) in chain(levels) {
            dimensions.push(Dimension::Vertical { from, to, at });
        }
        dimensions
    }
}

/// Consecutive stops, sorted, without repeats or zero-length steps.
fn chain(mut stops: Vec<Length>) -> Vec<(Length, Length)> {
    stops.sort();
    stops.dedup();
    stops.windows(2).map(|w| (w[0], w[1])).collect()
}

/// From a wall's start to its end, to the nanometer for walls along an
/// axis.
fn wall_length(wall: &Wall) -> Length {
    let dx = (wall.end.x - wall.start.x).nm();
    let dy = (wall.end.y - wall.start.y).nm();
    if dx == 0 || dy == 0 {
        return Length::from_nm(dx.abs() + dy.abs());
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "a wall is far below 2^53 nm"
    )]
    Length::from_nm((dx as f64).hypot(dy as f64).round() as i64)
}

/// What is wrong with where the project's products are placed on walls:
/// past a wall's end or above its top, over its openings, or into another
/// product, also around a corner. Products placed freely, such as islands,
/// are not checked yet.
#[must_use]
pub fn placement_problems(project: &Project) -> Vec<String> {
    let mut problems = Vec::new();
    for room in &project.rooms {
        let mut footprints: Vec<(&str, Footprint)> = Vec::new();
        for placement in &room.placements {
            let (Some(wall), Some(product)) = (
                placement
                    .wall
                    .and_then(|id| room.walls.iter().find(|w| w.id == id)),
                project.products.iter().find(|p| p.id == placement.product),
            ) else {
                continue;
            };
            let offset = placement.offset.unwrap_or_default();
            let elevation = placement.elevation.unwrap_or_default();
            let label = product.label.as_str();
            if offset < Length::ZERO || offset + product.width > wall_length(wall) {
                problems.push(format!("{label} runs past the end of wall {}", wall.name));
            }
            if elevation + product.height > wall.height {
                problems.push(format!("{label} is taller than wall {}", wall.name));
            }
            let rect = Rect {
                x: offset,
                y: elevation,
                width: product.width,
                height: product.height,
            };
            for opening in &wall.openings {
                let opening_rect = Rect {
                    x: opening.offset,
                    y: opening.sill_height,
                    width: opening.width,
                    height: opening.height,
                };
                if overlaps(rect, opening_rect) {
                    let kind = match &opening.kind {
                        WallOpeningKind::Window => "window",
                        WallOpeningKind::Door => "door",
                        WallOpeningKind::PassThrough | WallOpeningKind::Other(_) => "opening",
                    };
                    problems.push(format!("{label} covers the {kind} on wall {}", wall.name));
                }
            }
            footprints.push((label, Footprint::on_wall(wall, offset, elevation, product)));
        }
        for (i, (a, first)) in footprints.iter().enumerate() {
            for (b, second) in &footprints[i + 1..] {
                if first.collides(second) {
                    problems.push(format!("{a} and {b} overlap"));
                }
            }
        }
    }
    problems
}

/// Whether two rectangles share more than an edge.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// The space a product takes in a room: its outline in plan, in
/// millimeters, and its height above the floor.
#[derive(Clone, Debug, PartialEq)]
struct Footprint {
    corners: [(f64, f64); 4],
    bottom: f64,
    top: f64,
}

/// What two footprints may share without colliding, in millimeters.
const TOLERANCE: f64 = 0.001;

#[allow(clippy::cast_precision_loss, reason = "a room is far below 2^53 nm")]
fn mm(length: Length) -> f64 {
    length.nm() as f64 / 1_000_000.0
}

impl Footprint {
    fn on_wall(wall: &Wall, offset: Length, elevation: Length, product: &Product) -> Self {
        let start = (mm(wall.start.x), mm(wall.start.y));
        let (dx, dy) = (mm(wall.end.x) - start.0, mm(wall.end.y) - start.1);
        let length = dx.hypot(dy).max(f64::MIN_POSITIVE);
        let along = (dx / length, dy / length);
        // The wall's front face is to the left walking along it.
        let inward = (-along.1, along.0);
        let at = |a: f64, b: f64| {
            (
                start.0 + along.0 * a + inward.0 * b,
                start.1 + along.1 * a + inward.1 * b,
            )
        };
        let (left, right, depth) = (mm(offset), mm(offset + product.width), mm(product.depth));
        Self {
            corners: [
                at(left, 0.0),
                at(right, 0.0),
                at(right, depth),
                at(left, depth),
            ],
            bottom: mm(elevation),
            top: mm(elevation + product.height),
        }
    }

    /// Whether the two take some of the same space: they overlap in
    /// height and their outlines in plan overlap, by the separating axis
    /// test.
    fn collides(&self, other: &Self) -> bool {
        if self.top.min(other.top) - self.bottom.max(other.bottom) <= TOLERANCE {
            return false;
        }
        let axes = [self, other].into_iter().flat_map(|f| {
            (0..2).map(move |i| {
                let (a, b) = (f.corners[i], f.corners[i + 1]);
                let (x, y) = (b.0 - a.0, b.1 - a.1);
                let norm = x.hypot(y).max(f64::MIN_POSITIVE);
                (-y / norm, x / norm)
            })
        });
        for axis in axes {
            let project = |f: &Self| {
                f.corners
                    .iter()
                    .map(|c| c.0 * axis.0 + c.1 * axis.1)
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                        (lo.min(v), hi.max(v))
                    })
            };
            let (a, b) = (project(self), project(other));
            if a.1.min(b.1) - a.0.max(b.0) <= TOLERANCE {
                return false;
            }
        }
        true
    }
}
