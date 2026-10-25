// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Exact lengths as integer nanometers (ADR-0009).

use std::fmt;
use std::ops::{Add, Mul, Sub};

use serde::{Deserialize, Serialize};

const NM_PER_INCH: i64 = 25_400_000;

/// A signed length in nanometers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Length(i64);

impl Length {
    pub const ZERO: Self = Self(0);

    pub const fn from_nm(nm: i64) -> Self {
        Self(nm)
    }

    pub const fn nm(self) -> i64 {
        self.0
    }

    /// Splits `self` into `parts` lengths that differ by at most 1 nm and add
    /// up exactly to `self`; the first ones receive the remainder.
    pub fn split_evenly(self, parts: u32) -> Vec<Self> {
        assert!(parts > 0, "cannot split into zero parts");
        let parts_i64 = i64::from(parts);
        let base = self.0.div_euclid(parts_i64);
        let remainder = self.0.rem_euclid(parts_i64);
        (0..parts_i64)
            .map(|i| Self(base + i64::from(i < remainder)))
            .collect()
    }

    /// Floor of half the length.
    pub const fn half(self) -> Self {
        Self(self.0.div_euclid(2))
    }

    /// Formats as fractional inches rounded to the nearest `1/denominator`,
    /// ties away from zero: `30"`, `7/32"`, `23-7/32"`.
    pub fn to_fractional_inches(self, denominator: i64) -> FractionalInches {
        FractionalInches {
            units: round_half_away(
                i128::from(self.0) * i128::from(denominator),
                i128::from(NM_PER_INCH),
            ),
            denominator,
        }
    }
}

impl Add for Length {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Length {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Mul<i64> for Length {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self {
        Self(self.0 * rhs)
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A length expressed as a whole number of `1/denominator` inch units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FractionalInches {
    units: i128,
    denominator: i64,
}

impl fmt::Display for FractionalInches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.units < 0 { "-" } else { "" };
        let denominator = i128::from(self.denominator);
        let units = self.units.abs();
        let (whole, rest) = (units / denominator, units % denominator);
        if rest == 0 {
            return write!(f, "{sign}{whole}\"");
        }
        let common = gcd(rest, denominator);
        let (num, den) = (rest / common, denominator / common);
        if whole == 0 {
            write!(f, "{sign}{num}/{den}\"")
        } else {
            write!(f, "{sign}{whole}-{num}/{den}\"")
        }
    }
}

fn round_half_away(numerator: i128, denominator: i128) -> i128 {
    let negative = (numerator < 0) != (denominator < 0);
    let (n, d) = (numerator.abs(), denominator.abs());
    let magnitude = (2 * n + d) / (2 * d);
    if negative { -magnitude } else { magnitude }
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inches(nm: i64) -> String {
        Length::from_nm(nm).to_fractional_inches(32).to_string()
    }

    #[test]
    fn formats_whole_fraction_and_mixed_inches() {
        assert_eq!(inches(762_000_000), "30\"");
        assert_eq!(inches(5_556_250), "7/32\"");
        assert_eq!(inches(589_756_250), "23-7/32\"");
        assert_eq!(inches(0), "0\"");
        assert_eq!(inches(-5_556_250), "-7/32\"");
    }

    #[test]
    fn rounds_ties_away_from_zero() {
        // 1/64" is exactly half of 1/32".
        assert_eq!(inches(396_875), "1/32\"");
        assert_eq!(inches(-396_875), "-1/32\"");
        assert_eq!(inches(396_874), "0\"");
    }

    #[test]
    fn split_evenly_distributes_the_remainder_from_the_left() {
        let parts = Length::from_nm(781_446_875).split_evenly(2);
        assert_eq!(
            parts,
            [Length::from_nm(390_723_438), Length::from_nm(390_723_437)]
        );
    }

    #[test]
    fn split_evenly_always_adds_up() {
        for total in [0, 1, 7, 999_999_999, 781_446_875] {
            for parts in 1..=7 {
                let sum: i64 = Length::from_nm(total)
                    .split_evenly(parts)
                    .iter()
                    .map(|p| p.nm())
                    .sum();
                assert_eq!(sum, total, "total {total} into {parts}");
            }
        }
    }
}
