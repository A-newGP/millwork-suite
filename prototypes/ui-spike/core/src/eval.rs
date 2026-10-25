// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Deterministic evaluation of products into parts and operations, following
//! the rules in the spike README.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::length::Length;
use crate::model::{EdgeBand, HingeSide, Insert, Material, Product, Project, Standard};

/// Edge banding of a part's four edges, in the order x0, x1, y0, y1.
pub type Edges<'p> = [Option<&'p EdgeBand>; 4];

#[derive(Debug, Clone, PartialEq)]
pub struct Part<'p> {
    pub product: &'p str,
    pub key: String,
    pub material: &'p Material,
    pub quantity: u32,
    pub length: Length,
    pub width: Length,
    pub edges: Edges<'p>,
}

impl Part<'_> {
    pub fn thickness(&self) -> Length {
        self.material.actual_thickness
    }

    pub fn cut_length(&self) -> Length {
        self.length - band_thickness(self.edges[0]) - band_thickness(self.edges[1])
    }

    pub fn cut_width(&self) -> Length {
        self.width - band_thickness(self.edges[2]) - band_thickness(self.edges[3])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hole<'p> {
    pub product: &'p str,
    pub part: String,
    pub x: Length,
    pub y: Length,
    pub diameter: Length,
    pub depth: Length,
}

#[derive(Debug, Default, PartialEq)]
pub struct Evaluation<'p> {
    pub parts: Vec<Part<'p>>,
    pub holes: Vec<Hole<'p>>,
}

/// Evaluates every product of the project, in file order.
pub fn evaluate(project: &Project) -> Result<Evaluation<'_>> {
    let library = Library::index(project);
    let mut evaluation = Evaluation::default();
    for product in &project.products {
        evaluate_product(product, &library, &mut evaluation)?;
    }
    Ok(evaluation)
}

struct Library<'p> {
    materials: HashMap<&'p str, &'p Material>,
    edge_bands: HashMap<&'p str, &'p EdgeBand>,
    standards: HashMap<&'p str, &'p Standard>,
}

impl<'p> Library<'p> {
    fn index(project: &'p Project) -> Self {
        let lib = &project.library;
        Self {
            materials: lib.materials.iter().map(|m| (m.id.as_str(), m)).collect(),
            edge_bands: lib.edge_bands.iter().map(|b| (b.id.as_str(), b)).collect(),
            standards: lib.standards.iter().map(|s| (s.id.as_str(), s)).collect(),
        }
    }

    fn get<T>(map: &HashMap<&'p str, &'p T>, id: &str, product: &Product) -> Result<&'p T> {
        map.get(id).copied().ok_or_else(|| {
            Error::InvalidProject(format!("{}: unknown reference {id}", product.label))
        })
    }
}

fn evaluate_product<'p>(
    product: &'p Product,
    lib: &Library<'p>,
    out: &mut Evaluation<'p>,
) -> Result<()> {
    let std = Library::get(&lib.standards, &product.standard, product)?;
    let roles = &product.materials;
    let case = Library::get(&lib.materials, &roles.case, product)?;
    let back = Library::get(&lib.materials, &roles.back, product)?;
    let doors = Library::get(&lib.materials, &roles.doors, product)?;
    let case_band = Library::get(&lib.edge_bands, &roles.edge_band_case, product)?;
    let front_band = Library::get(&lib.edge_bands, &roles.edge_band_fronts, product)?;

    let (w, d) = (product.width, product.depth);
    let t = case.actual_thickness;
    let box_height = product.height - std.toe_kick.height;
    let gap = std.fronts.gap;
    let groove = std.back.groove_depth;
    let inner_width = w - t * 2;

    let front_y1 = [None, None, None, Some(case_band)];
    let unbanded = [None; 4];
    let part = |key: &str, material, length, width, edges| Part {
        product: &product.label,
        key: key.to_owned(),
        material,
        quantity: product.quantity,
        length,
        width,
        edges,
    };
    let mut parts = vec![
        part("case/side.left", case, box_height, d, front_y1),
        part("case/side.right", case, box_height, d, front_y1),
        part("case/bottom", case, inner_width, d, front_y1),
        part(
            "case/stretcher.front",
            case,
            inner_width,
            std.stretchers.width,
            front_y1,
        ),
        part(
            "case/stretcher.back",
            case,
            inner_width,
            std.stretchers.width,
            unbanded,
        ),
        part(
            "case/back",
            back,
            box_height - t + groove,
            inner_width + groove * 2,
            unbanded,
        ),
    ];

    let Insert::Doors {
        count, hinge_side, ..
    } = &product.front.insert;
    let hinge_sides = match count {
        1 => vec![hinge_side.unwrap_or_default()],
        2 => vec![HingeSide::Left, HingeSide::Right],
        _ => {
            return Err(Error::InvalidProject(format!(
                "{}: unsupported door count {count}",
                product.label
            )));
        }
    };
    let door_length = box_height - gap;
    let door_widths = (w - gap - gap * i64::from(count - 1)).split_evenly(*count);

    for (index, (&door_width, side)) in door_widths.iter().zip(hinge_sides).enumerate() {
        let key = format!("front/door.{}", index + 1);
        out.holes.extend(hinge_holes(
            product,
            std,
            &key,
            door_length,
            door_width,
            side,
        ));
        parts.push(part(
            &key,
            doors,
            door_length,
            door_width,
            [Some(front_band); 4],
        ));
    }

    if let Some(bad) = parts
        .iter()
        .find(|p| p.length <= Length::ZERO || p.width <= Length::ZERO)
    {
        return Err(Error::InvalidProject(format!(
            "{}: non-positive size for {}",
            product.label, bad.key
        )));
    }
    out.parts.extend(parts);
    Ok(())
}

fn hinge_holes<'p>(
    product: &'p Product,
    std: &Standard,
    key: &str,
    door_length: Length,
    door_width: Length,
    side: HingeSide,
) -> Vec<Hole<'p>> {
    let hinges = &std.hinges;
    let y = match side {
        HingeSide::Left => hinges.edge_distance,
        HingeSide::Right => door_width - hinges.edge_distance,
    };
    let mut xs = vec![hinges.end_distance, door_length - hinges.end_distance];
    if door_length > hinges.third_hinge_above {
        xs.push(door_length.half());
    }
    xs.sort_unstable();
    xs.into_iter()
        .map(|x| Hole {
            product: &product.label,
            part: key.to_owned(),
            x,
            y,
            diameter: hinges.cup_diameter,
            depth: hinges.cup_depth,
        })
        .collect()
}

fn band_thickness(band: Option<&EdgeBand>) -> Length {
    band.map_or(Length::ZERO, |b| b.thickness)
}
