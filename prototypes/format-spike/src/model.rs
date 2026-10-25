// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The authored data of RFC-0001 as typed Rust values.
//!
//! Every object keeps the fields this version does not know in `unknown`,
//! and every open-ended enum keeps values it does not know in `Other`, so a
//! file written by a newer minor version or a plugin survives a save
//! (invariant 7). Struct field order is the order fields are written in.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::units::{Angle, Length};

/// Fields this version does not know, written back unchanged after the
/// known ones, in key order.
pub type Unknown = BTreeMap<String, Value>;

/// Plugin data keyed by a reverse-domain namespace (RFC-0001 §11).
pub type Extensions = BTreeMap<String, Value>;

// `project.json`

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub settings: Settings,
    #[serde(default)]
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default)]
    pub revisions: Vec<Revision>,
    #[serde(default)]
    pub extensions: Extensions,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub units: Units,
    /// Display precision, itself a length: 1/32" or 0.1 mm.
    pub precision: Length,
    pub default_standard: Uuid,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Units {
    Imperial,
    Metric,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub label: String,
    pub date: String,
    pub author: String,
    pub description: String,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub walls: Vec<Wall>,
    #[serde(default)]
    pub placements: Vec<Placement>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: Length,
    pub y: Length,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: Length,
    pub y: Length,
    pub z: Length,
}

/// A wall in plan. Its front side is on the left walking from start to end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wall {
    pub id: Uuid,
    pub name: String,
    pub start: Point,
    pub end: Point,
    pub height: Length,
    pub thickness: Length,
    #[serde(default)]
    pub openings: Vec<WallOpening>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallOpening {
    pub id: Uuid,
    pub kind: WallOpeningKind,
    pub offset: Length,
    pub width: Length,
    pub height: Length,
    pub sill_height: Length,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WallOpeningKind {
    Window,
    Door,
    PassThrough,
    #[serde(untagged)]
    Other(String),
}

/// Puts a product in a room: on a wall (`wall`, `offset`, `elevation`) or
/// free (`position`, `rotation`). Validation checks that exactly one form is
/// used.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub id: Uuid,
    pub product: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elevation: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Point3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Angle>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: Uuid,
    pub label: String,
    /// Product type and major version, e.g. `base-cabinet@1`.
    #[serde(rename = "type")]
    pub product_type: String,
    pub standard: Uuid,
    pub width: Length,
    pub height: Length,
    pub depth: Length,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,
    /// Typed by the product type's declaration, which this spike does not
    /// have, so values stay plain JSON.
    #[serde(default)]
    pub parameters: BTreeMap<String, Value>,
    /// Material role to library item.
    #[serde(default)]
    pub materials: BTreeMap<String, Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front: Option<FrontNode>,
    #[serde(default)]
    pub overrides: Vec<Override>,
    #[serde(default)]
    pub extensions: Extensions,
    #[serde(flatten)]
    pub unknown: Unknown,
}

/// A node of the front split tree: a `split` with `children`, or an
/// opening with an `insert`. Validation checks that it is exactly one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontNode {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<SplitDirection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FrontNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert: Option<Insert>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

/// A child's size within its split: `{"length": nm}` or `{"share": n}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Size {
    Length(Length),
    Share(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Insert {
    pub kind: InsertKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<FrontStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hinge_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelves: Option<u8>,
    /// Changes to the hardware the standard places (RFC-0001 §6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjustments: Vec<Adjustment>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InsertKind {
    Doors,
    Drawer,
    FalseFront,
    Open,
    Appliance,
    None,
    #[serde(untagged)]
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrontStyle {
    Slab,
    FivePiece,
    #[serde(untagged)]
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adjustment {
    pub id: Uuid,
    /// Key of the derived hardware, e.g. `hinges/left`.
    pub target: String,
    pub action: AdjustmentAction,
    /// Distance along the insert edge, for `add` and `move`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Length>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdjustmentAction {
    Add,
    Move,
    Remove,
    #[serde(untagged)]
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Override {
    pub id: Uuid,
    /// Key of the derived part or assembly, e.g. `case/bottom`.
    pub target: String,
    pub action: OverrideAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    pub note: String,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverrideAction {
    Set,
    AddOperation,
    RemoveOperation,
    Exclude,
    AddPart,
    #[serde(untagged)]
    Other(String),
}

// `library.json`

/// Snapshots of every library item the project uses (RFC-0001 §8). An
/// item's `id` and `version` are those of the shop's master copy.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    #[serde(default)]
    pub materials: Vec<SheetMaterial>,
    #[serde(default)]
    pub edge_bands: Vec<EdgeBand>,
    #[serde(default)]
    pub hardware: Vec<Hardware>,
    #[serde(default)]
    pub standards: Vec<Standard>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetMaterial {
    pub id: Uuid,
    pub version: u32,
    pub name: String,
    pub nominal_thickness: Length,
    pub actual_thickness: Length,
    pub length: Length,
    pub width: Length,
    pub grain: Grain,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Grain {
    None,
    Length,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeBand {
    pub id: Uuid,
    pub version: u32,
    pub name: String,
    pub thickness: Length,
    pub width: Length,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hardware {
    pub id: Uuid,
    pub version: u32,
    pub category: String,
    pub name: String,
    /// The dimensions the evaluator needs, e.g. `cupDiameter`.
    #[serde(default)]
    pub dimensions: BTreeMap<String, Length>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Standard {
    pub id: Uuid,
    pub version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Uuid>,
    pub method: Method,
    /// Rule groups of RFC-0001 §7, kept as plain JSON: their schema belongs
    /// to the format specification, not to this spike.
    #[serde(default)]
    pub rules: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub unknown: Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    Frameless,
    FaceFrame,
}
