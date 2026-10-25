// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the window shows and edits, without any window: the cabinet being
//! sized and the units it is shown in. Keeping it free of Slint lets it be
//! tested directly.

use opencabinet_model::{Length, Units};

/// One of the cabinet's overall dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    /// Left to right.
    Width,
    /// Floor to top.
    Height,
    /// Front to back.
    Depth,
}

impl Dimension {
    /// Every dimension, in the order the window shows them.
    pub const ALL: [Self; 3] = [Self::Width, Self::Height, Self::Depth];
}

/// The editing state of the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Designer {
    units: Units,
    width: Length,
    height: Length,
    depth: Length,
}

impl Default for Designer {
    /// A 30" × 34-1/2" × 24" base cabinet, in inches.
    fn default() -> Self {
        Self {
            units: Units::Imperial,
            width: Length::from_inches(30),
            height: Length::from_inch_fraction(69, 2).expect("a valid fraction"),
            depth: Length::from_inches(24),
        }
    }
}

impl Designer {
    /// The units lengths are typed and shown in.
    #[must_use]
    pub fn units(&self) -> Units {
        self.units
    }

    /// Changes the units lengths are shown in; stored values never change.
    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    /// A dimension's stored value.
    #[must_use]
    pub fn get(&self, dimension: Dimension) -> Length {
        match dimension {
            Dimension::Width => self.width,
            Dimension::Height => self.height,
            Dimension::Depth => self.depth,
        }
    }

    /// Reads what the user typed into a dimension and stores it.
    ///
    /// # Errors
    /// A message for the user when the text is not a positive length; the
    /// stored value is then left unchanged.
    pub fn edit(&mut self, dimension: Dimension, text: &str) -> Result<(), String> {
        let length = Length::parse(text, self.units).map_err(|e| capitalize(&e.to_string()))?;
        if length <= Length::ZERO {
            return Err("Must be greater than zero".into());
        }
        match dimension {
            Dimension::Width => self.width = length,
            Dimension::Height => self.height = length,
            Dimension::Depth => self.depth = length,
        }
        Ok(())
    }

    /// A dimension as text to put in its field, in the current units.
    #[must_use]
    pub fn text(&self, dimension: Dimension) -> String {
        self.get(dimension).show(self.units, self.precision()).text
    }

    /// How a dimension was read, for the line under its field: `= 30-1/2"`,
    /// or `≈ 30-1/2"` when the display precision rounds it.
    #[must_use]
    pub fn reading(&self, dimension: Dimension) -> String {
        let shown = self.get(dimension).show(self.units, self.precision());
        let sign = if shown.exact { '=' } else { '≈' };
        format!("{sign} {}", shown.text)
    }

    /// Width over height, for drawing the front in proportion.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "a ratio for the screen"
    )]
    pub fn front_ratio(&self) -> f32 {
        (self.width.nm() as f64 / self.height.nm() as f64) as f32
    }

    /// The display precision: 1/32" or 0.1 mm, the usual shop precisions.
    fn precision(&self) -> Length {
        match self.units {
            Units::Imperial => Length::from_inch_fraction(1, 32).expect("a valid fraction"),
            Units::Metric => Length::from_nm(100_000),
        }
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_store_exact_values_and_keep_the_last_good_one() {
        let mut designer = Designer::default();
        designer.edit(Dimension::Width, "30 1/2").unwrap();
        assert_eq!(designer.get(Dimension::Width), Length::from_nm(774_700_000));
        assert_eq!(designer.reading(Dimension::Width), "= 30-1/2\"");

        assert_eq!(
            designer.edit(Dimension::Width, "thirty"),
            Err("Not a length".into())
        );
        assert_eq!(
            designer.edit(Dimension::Width, "0"),
            Err("Must be greater than zero".into())
        );
        assert_eq!(designer.get(Dimension::Width), Length::from_nm(774_700_000));
    }

    #[test]
    fn switching_units_changes_only_the_display() {
        let mut designer = Designer::default();
        designer.set_units(Units::Metric);
        assert_eq!(designer.text(Dimension::Width), "762 mm");
        assert_eq!(designer.reading(Dimension::Height), "= 876.3 mm");
        designer.edit(Dimension::Depth, "600").unwrap();
        designer.set_units(Units::Imperial);
        // 600 mm is 23.622…": shown rounded, but stored exactly.
        assert_eq!(designer.reading(Dimension::Depth), "≈ 23-5/8\"");
        assert_eq!(designer.get(Dimension::Depth), Length::from_mm(600));
    }

    #[test]
    fn the_front_ratio_follows_width_and_height() {
        let mut designer = Designer::default();
        designer.edit(Dimension::Width, "69/2").unwrap();
        assert!((designer.front_ratio() - 1.0).abs() < f32::EPSILON);
    }
}
