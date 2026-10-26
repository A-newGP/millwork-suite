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
