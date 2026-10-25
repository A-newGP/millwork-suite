// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Exact quantities stored as integers, never as floating point.

use serde::{Deserialize, Serialize};

/// A length in integer nanometers (ADR-0009), stored as a JSON integer.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Length(pub i64);

impl Length {
    pub const NM_PER_INCH: i64 = 25_400_000;

    /// `num / den` inches; exact for every denominator up to 64.
    pub const fn inches(num: i64, den: i64) -> Self {
        Self(num * Self::NM_PER_INCH / den)
    }

    pub const fn mm(mm: i64) -> Self {
        Self(mm * 1_000_000)
    }
}

/// An angle in integer microdegrees, counterclockwise seen from above.
///
/// RFC-0001 does not define angles yet; integers keep rotations exact and
/// deterministic like lengths. 90° is `90_000_000`.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Angle(pub i64);

impl Angle {
    pub const FULL_TURN: i64 = 360_000_000;

    pub const fn degrees(deg: i64) -> Self {
        Self(deg * 1_000_000)
    }
}
