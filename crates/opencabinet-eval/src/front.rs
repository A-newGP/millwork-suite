// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Divides a product's front into the openings of its split tree
//! (RFC-0001 §6).

use opencabinet_model::{FrontNode, Insert, Length, Size, SplitDirection};

/// A rectangle on the product's front: `x` from the left side, `y` up from
/// the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: Length,
    /// Bottom edge.
    pub y: Length,
    /// Width.
    pub width: Length,
    /// Height.
    pub height: Length,
}

impl Rect {
    /// This rectangle shrunk by `gap` in each direction: half on one side,
    /// the rest on the other, so neighbors end exactly `gap` apart even when
    /// it is an odd number of nanometers.
    #[must_use]
    pub fn inset(self, gap: Length) -> Self {
        let half = gap.div_round(2);
        Self {
            x: self.x + half,
            y: self.y + half,
            width: self.width - gap,
            height: self.height - gap,
        }
    }
}

/// An opening of the front with its insert and where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening<'p> {
    /// The tree node, whose identity makes part keys stable.
    pub node: &'p FrontNode,
    /// What fills it.
    pub insert: &'p Insert,
    /// Where it is, before any gap.
    pub rect: Rect,
}

/// The openings of a front tree laid over `rect`, top to bottom and left to
/// right.
///
/// Fixed sizes are taken first; the space left is divided among the
/// shares in proportion, and the nanometers left over by that division go
/// one each to the first shares, so the sizes always add up to the whole.
///
/// # Errors
/// When the fixed sizes do not fit, or a node is neither a split with
/// children nor an opening with an insert.
pub fn openings(root: &FrontNode, rect: Rect) -> Result<Vec<Opening<'_>>, String> {
    let mut out = Vec::new();
    lay_out(root, rect, &mut out)?;
    Ok(out)
}

fn lay_out<'p>(node: &'p FrontNode, rect: Rect, out: &mut Vec<Opening<'p>>) -> Result<(), String> {
    match (&node.split, &node.insert) {
        (None, Some(insert)) if node.children.is_empty() => {
            out.push(Opening { node, insert, rect });
            Ok(())
        }
        (Some(direction), None) if !node.children.is_empty() => {
            let total = match direction {
                SplitDirection::Horizontal => rect.height,
                SplitDirection::Vertical => rect.width,
            };
            let sizes = divide(&node.children, total)?;
            // Horizontal splits run from the top down; vertical ones from
            // the left.
            let mut top = rect.y + rect.height;
            let mut left = rect.x;
            for (child, size) in node.children.iter().zip(sizes) {
                let child_rect = match direction {
                    SplitDirection::Horizontal => {
                        top -= size;
                        Rect {
                            y: top,
                            height: size,
                            ..rect
                        }
                    }
                    SplitDirection::Vertical => {
                        let r = Rect {
                            x: left,
                            width: size,
                            ..rect
                        };
                        left += size;
                        r
                    }
                };
                lay_out(child, child_rect, out)?;
            }
            Ok(())
        }
        _ => Err(format!(
            "front node {} must be a split with children or an insert",
            node.id
        )),
    }
}

/// The children's sizes along the split, adding up to `total` exactly.
fn divide(children: &[FrontNode], total: Length) -> Result<Vec<Length>, String> {
    let fixed: Length = children
        .iter()
        .filter_map(|c| match c.size {
            Some(Size::Length(length)) => Some(length),
            _ => None,
        })
        .sum();
    let shares: u64 = children
        .iter()
        .filter_map(|c| match c.size {
            Some(Size::Share(share)) => Some(u64::from(share)),
            _ => None,
        })
        .sum();
    let rest = total - fixed;
    if rest < Length::ZERO {
        return Err("the front's fixed sizes are larger than the opening".into());
    }
    if shares == 0 && rest != Length::ZERO {
        return Err("the front's fixed sizes do not fill the opening".into());
    }

    // Each share gets rest × share / shares, rounded down; the remainder
    // goes one nanometer at a time to the first shares.
    let rest_nm = i128::from(rest.nm());
    let mut given = 0;
    let mut sizes: Vec<i128> = children
        .iter()
        .map(|c| match c.size {
            Some(Size::Length(length)) => Ok(i128::from(length.nm())),
            Some(Size::Share(share)) => {
                let part = rest_nm * i128::from(share) / i128::from(shares);
                given += part;
                Ok(part)
            }
            None => Err(format!("front node {} needs a size", c.id)),
        })
        .collect::<Result<_, _>>()?;
    let mut left_over = rest_nm - given;
    for (child, size) in children.iter().zip(&mut sizes) {
        if left_over == 0 {
            break;
        }
        if matches!(child.size, Some(Size::Share(s)) if s > 0) {
            *size += 1;
            left_over -= 1;
        }
    }
    Ok(sizes
        .into_iter()
        .map(|nm| Length::from_nm(i64::try_from(nm).expect("within the opening")))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencabinet_model::{Id, InsertKind, Unknown};

    fn leaf(size: Size) -> FrontNode {
        FrontNode {
            id: Id::new(),
            size: Some(size),
            split: None,
            children: Vec::new(),
            insert: Some(Insert {
                kind: InsertKind::Drawer,
                style: None,
                count: None,
                hinge_side: None,
                shelves: None,
                adjustments: Vec::new(),
                unknown: Unknown::new(),
            }),
            unknown: Unknown::new(),
        }
    }

    fn split(direction: SplitDirection, children: Vec<FrontNode>) -> FrontNode {
        FrontNode {
            id: Id::new(),
            size: None,
            split: Some(direction),
            children,
            insert: None,
            unknown: Unknown::new(),
        }
    }

    fn rect(width: i64, height: i64) -> Rect {
        Rect {
            x: Length::ZERO,
            y: Length::from_nm(100),
            width: Length::from_nm(width),
            height: Length::from_nm(height),
        }
    }

    #[test]
    fn fixed_sizes_come_first_and_shares_split_the_rest_exactly() {
        let tree = split(
            SplitDirection::Horizontal,
            vec![
                leaf(Size::Length(Length::from_nm(30))),
                leaf(Size::Share(1)),
                leaf(Size::Share(2)),
            ],
        );
        let found = openings(&tree, rect(500, 101)).unwrap();
        let heights: Vec<i64> = found.iter().map(|o| o.rect.height.nm()).collect();
        // 71 left: 23.67 and 47.33 round down to 23 and 47; the extra
        // nanometer goes to the first share.
        assert_eq!(heights, [30, 24, 47]);
        // From the top down: the fixed one is highest.
        let bottoms: Vec<i64> = found.iter().map(|o| o.rect.y.nm()).collect();
        assert_eq!(bottoms, [171, 147, 100]);
    }

    #[test]
    fn vertical_splits_run_from_the_left() {
        let tree = split(
            SplitDirection::Vertical,
            vec![leaf(Size::Share(1)), leaf(Size::Share(1))],
        );
        let found = openings(&tree, rect(301, 50)).unwrap();
        let lefts: Vec<(i64, i64)> = found
            .iter()
            .map(|o| (o.rect.x.nm(), o.rect.width.nm()))
            .collect();
        assert_eq!(lefts, [(0, 151), (151, 150)]);
    }

    #[test]
    fn sizes_that_do_not_fit_are_errors() {
        let too_big = split(
            SplitDirection::Horizontal,
            vec![leaf(Size::Length(Length::from_nm(200)))],
        );
        assert!(openings(&too_big, rect(10, 100)).is_err());
        let too_small = split(
            SplitDirection::Horizontal,
            vec![leaf(Size::Length(Length::from_nm(20)))],
        );
        assert!(openings(&too_small, rect(10, 100)).is_err());
    }

    #[test]
    fn neighbors_end_one_gap_apart_even_for_an_odd_gap() {
        let gap = Length::from_nm(3);
        let lower = rect(100, 50).inset(gap);
        let upper = Rect {
            y: Length::from_nm(150),
            ..rect(100, 50)
        }
        .inset(gap);
        assert_eq!(upper.y - (lower.y + lower.height), gap);
    }
}
