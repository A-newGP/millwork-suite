// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The rule groups of a construction standard (RFC-0001 §7) that the
//! evaluator reads, typed from the standard's JSON.
//!
//! This is the evaluator's first version of the schema, which the format
//! specification will fix. Groups the evaluator does not use yet, such as
//! `systemHoles`, are ignored here and kept in the file.

use opencabinet_model::{Length, Standard};
use serde::Deserialize;

/// The rules a frameless cabinet needs.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    /// The base the box stands on.
    pub toe_kick: ToeKick,
    /// The box.
    pub case: Case,
    /// The back panel.
    pub back: Back,
    /// Doors and drawer fronts.
    pub fronts: Fronts,
    /// Drawer boxes; needed only by cabinets with drawers.
    #[serde(default)]
    pub drawers: Option<Drawers>,
    /// Adjustable shelves; needed only by cabinets with shelves.
    #[serde(default)]
    pub shelves: Option<Shelves>,
    /// Five-piece doors and fronts; needed only by fronts in that style.
    #[serde(default)]
    pub five_piece: Option<FivePiece>,
}

/// Five-piece doors and fronts: two stiles, two rails and a floating panel.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FivePiece {
    /// The width of the stiles, the vertical members.
    pub stile_width: Length,
    /// The width of the rails, the horizontal members.
    pub rail_width: Length,
    /// How far each end of a rail reaches into a stile: the sticking depth
    /// for cope-and-stick, the tenon length for mortise-and-tenon.
    pub joint_depth: Length,
    /// How deep the groove that holds the panel is.
    pub panel_groove_depth: Length,
    /// How much smaller the panel is than its grooves, in total, so it can
    /// move with the seasons.
    pub panel_clearance: Length,
}

/// Drawer boxes: two sides run the full depth and hold the front and back
/// between them; the bottom sits in a groove in all four.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drawers {
    /// The room the slide needs on each side of the box.
    pub side_clearance: Length,
    /// How much lower the box is than its opening.
    pub vertical_clearance: Length,
    /// The room left behind the box at the back of the cabinet.
    pub rear_clearance: Length,
    /// The slide lengths the shop stocks; the box is as deep as the longest
    /// one that fits.
    pub slide_lengths: Vec<Length>,
    /// How deep the bottom's groove is.
    pub bottom_groove_depth: Length,
}

/// Adjustable shelves, which span the cabinet's inside width.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shelves {
    /// How much narrower than the inside a shelf is, in total.
    pub side_clearance: Length,
    /// How far back from the front edge of the case a shelf starts.
    pub front_setback: Length,
    /// The depth the back takes, which a shelf stops short of.
    pub back_allowance: Length,
}

/// The toe kick. The box stands on a separate base of this height, which
/// is not part of the cabinet.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToeKick {
    /// From the floor to the bottom of the box.
    pub height: Length,
}

/// The box: sides run full height and capture the bottom; two stretchers
/// tie the top.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Case {
    /// The width of the front and back stretchers.
    pub stretcher_width: Length,
}

/// How the back is built.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "construction", rename_all = "camelCase")]
pub enum Back {
    /// Set into grooves in the sides and the bottom.
    #[serde(rename_all = "camelCase")]
    Grooved {
        /// How deep the grooves are.
        groove_depth: Length,
    },
    /// Fastened over the back edges of the box.
    Applied,
}

/// How doors and drawer fronts sit on the box.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fronts {
    /// Only full overlay is evaluated so far.
    pub overlay: Overlay,
    /// Between neighboring fronts; half of it around the outside.
    pub gap: Length,
}

/// How much of the box the fronts cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Overlay {
    /// Fronts cover the whole box face.
    Full,
    /// Fronts cover part of the box edges.
    Partial,
    /// Fronts sit inside the opening.
    Inset,
}

impl Rules {
    /// Reads the rules a standard needs for evaluation.
    ///
    /// # Errors
    /// A message naming the missing or malformed rule.
    pub fn of(standard: &Standard) -> Result<Self, String> {
        serde_json::to_value(&standard.rules)
            .and_then(serde_json::from_value)
            .map_err(|e| format!("standard {}: {e}", standard.name))
    }
}
