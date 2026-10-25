// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The subset of the RFC-0001 data model used by the spike.

use serde::{Deserialize, Serialize};

use crate::length::Length;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub settings: Settings,
    pub library: Library,
    pub products: Vec<Product>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub units: String,
    pub precision_denominator: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub materials: Vec<Material>,
    pub edge_bands: Vec<EdgeBand>,
    pub standards: Vec<Standard>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: String,
    pub name: String,
    pub nominal_thickness: Length,
    pub actual_thickness: Length,
    pub sheet_length: Length,
    pub sheet_width: Length,
    pub grain: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeBand {
    pub id: String,
    pub name: String,
    pub thickness: Length,
    pub width: Length,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Standard {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub method: String,
    pub toe_kick: ToeKick,
    pub back: Back,
    pub stretchers: Stretchers,
    pub fronts: Fronts,
    pub hinges: Hinges,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToeKick {
    pub height: Length,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Back {
    pub groove_depth: Length,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stretchers {
    pub width: Length,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fronts {
    pub gap: Length,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hinges {
    pub cup_diameter: Length,
    pub cup_depth: Length,
    pub edge_distance: Length,
    pub end_distance: Length,
    pub third_hinge_above: Length,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub product_type: String,
    pub standard: String,
    pub width: Length,
    pub height: Length,
    pub depth: Length,
    pub quantity: u32,
    pub materials: MaterialRoles,
    pub front: FrontNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialRoles {
    pub case: String,
    pub back: String,
    pub doors: String,
    pub edge_band_case: String,
    pub edge_band_fronts: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontNode {
    pub id: String,
    pub insert: Insert,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Insert {
    #[serde(rename_all = "camelCase")]
    Doors {
        count: u32,
        style: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hinge_side: Option<HingeSide>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HingeSide {
    #[default]
    Left,
    Right,
}
