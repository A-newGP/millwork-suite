// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The evaluator against the sample kitchen in `fixtures/`, with sizes
//! worked out by hand from the formulas in the crate documentation.
//!
//! The sample's B30 is 30" × 34-1/2" × 24" with a 6" drawer over a pair of
//! doors: case of 18 mm melamine, back, drawer box and bottom of 12 mm
//! birch, fronts of 18 mm maple banded with 0.5 mm. Its standard: toe kick
//! K = 4", stretchers S = 4", front gap G = 1/8", back groove 3/8"; drawer
//! side clearance 1/2", vertical clearance 1-1/2", rear clearance 1",
//! slides from 12" to 22" by 2", bottom groove 1/4"; shelves 1/16"
//! narrower than the inside, set back 1/4" in front and 3/4" for the back.
//! So Hb = 34-1/2" − 4" = 774.7 mm and the inside is W − 2T = 726 mm.

use std::path::PathBuf;

use opencabinet_eval::{Part, evaluate};
use opencabinet_model::{FrontNode, FrontStyle, Length, Library, Project};
use serde::de::DeserializeOwned;

fn read<T: DeserializeOwned>(path: &str) -> T {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/kitchen.ocab")
        .join(path);
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// The sample kitchen, with the B30's five-piece doors, which are not
/// evaluated yet, made slabs.
fn kitchen_with_slab_doors() -> (Project, Library) {
    let mut project: Project = read("project.json");
    let front = project.products[0].front.as_mut().unwrap();
    front.children[1].insert.as_mut().unwrap().style = Some(FrontStyle::Slab);
    (project, read("library.json"))
}

/// Millimeters with only the decimals they need.
fn mm(nm: i64) -> String {
    let text = format!("{}.{:06}", nm / 1_000_000, (nm % 1_000_000).abs());
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// `key: length × width × thickness, cut length × cut width`, in mm, with
/// the front node ids shortened to their last three digits.
fn line(part: &Part) -> String {
    let key = part.key.replace("0199f8a0-0000-7000-8000-000000000", "");
    format!(
        "{key}: {} × {} × {}, cut {} × {}",
        mm(part.length.nm()),
        mm(part.width.nm()),
        mm(part.thickness.nm()),
        mm(part.cut_length.nm()),
        mm(part.cut_width.nm()),
    )
}

/// The B30's parts whose key starts with `prefix`.
fn b30(project: &Project, library: &Library, prefix: &str) -> Vec<String> {
    let id = project.products[0].id;
    evaluate(project, library)
        .parts
        .iter()
        .filter(|p| p.product == id && line(p).starts_with(prefix))
        .map(line)
        .collect()
}

#[test]
fn the_case_matches_the_sizes_worked_out_by_hand() {
    let (project, library) = kitchen_with_slab_doors();
    assert_eq!(
        b30(&project, &library, "case/"),
        [
            "case/side.left: 774.7 × 609.6 × 18, cut 774.7 × 609.6",
            "case/side.right: 774.7 × 609.6 × 18, cut 774.7 × 609.6",
            "case/bottom: 726 × 609.6 × 18, cut 726 × 609.6",
            "case/stretcher.front: 726 × 101.6 × 18, cut 726 × 101.6",
            "case/stretcher.back: 726 × 101.6 × 18, cut 726 × 101.6",
            // Hb − T + g by W − 2T + 2g.
            "case/back: 766.225 × 745.05 × 12, cut 766.225 × 745.05",
        ]
    );
    let all = evaluate(&project, &library);
    let id = project.products[0].id;
    // Placed twice in the kitchen.
    assert!(
        all.parts
            .iter()
            .filter(|p| p.product == id)
            .all(|p| p.quantity == 2)
    );
}

#[test]
fn the_fronts_match_the_sizes_worked_out_by_hand() {
    let (project, library) = kitchen_with_slab_doors();
    let fronts: Vec<String> = b30(&project, &library, "front/")
        .into_iter()
        .filter(|l| !l.contains("/box."))
        .collect();
    assert_eq!(
        fronts,
        [
            // The 6" opening less the gap, grain across: W − G by 6" − G,
            // cut 1 mm smaller for the bands.
            "front/402/drawer/front: 758.825 × 149.225 × 18, cut 757.825 × 148.225",
            // The doors share Hb − 6" = 622.3 mm, in two halves of 381 mm.
            "front/403/door.1: 619.125 × 377.825 × 18, cut 618.125 × 376.825",
            "front/403/door.2: 619.125 × 377.825 × 18, cut 618.125 × 376.825",
        ]
    );
}

#[test]
fn the_drawer_box_matches_the_sizes_worked_out_by_hand() {
    let (project, library) = kitchen_with_slab_doors();
    assert_eq!(
        b30(&project, &library, "front/402/drawer/box."),
        [
            // 24" − 1" leaves 23": the 22" slide, 558.8 mm, sets the depth.
            // The 6" opening less 1-1/2" is 114.3 mm high.
            "front/402/drawer/box.side.left: 558.8 × 114.3 × 12, cut 558.8 × 114.3",
            "front/402/drawer/box.side.right: 558.8 × 114.3 × 12, cut 558.8 × 114.3",
            // 726 − 2 × 12.7 = 700.6 mm wide outside, less two 12 mm sides.
            "front/402/drawer/box.front: 676.6 × 114.3 × 12, cut 676.6 × 114.3",
            "front/402/drawer/box.back: 676.6 × 114.3 × 12, cut 676.6 × 114.3",
            // The inside plus two 1/4" grooves each way.
            "front/402/drawer/box.bottom: 689.3 × 547.5 × 12, cut 689.3 × 547.5",
        ]
    );
}

#[test]
fn shelves_span_the_inside_less_their_clearances() {
    let (mut project, library) = kitchen_with_slab_doors();
    let front = project.products[0].front.as_mut().unwrap();
    front.children[1].insert.as_mut().unwrap().shelves = Some(2);
    assert_eq!(
        b30(&project, &library, "front/403/shelf"),
        [
            // 726 − 1/16" by 24" − 1/4" − 3/4".
            "front/403/shelf.1: 724.4125 × 584.2 × 18, cut 724.4125 × 584.2",
            "front/403/shelf.2: 724.4125 × 584.2 × 18, cut 724.4125 × 584.2",
        ]
    );
}

/// The sample's B30 turned into another kind of cabinet, `w` × `h` × `d`
/// inches, with a pair of slab doors filling its front.
fn variant(product_type: &str, w: i64, h: i64, d: i64) -> (Project, Library) {
    let (mut project, library) = kitchen_with_slab_doors();
    let product = &mut project.products[0];
    product.product_type = product_type.into();
    product.width = Length::from_inches(w);
    product.height = Length::from_inches(h);
    product.depth = Length::from_inches(d);
    let front = product.front.as_mut().unwrap();
    let doors = front.children.remove(1);
    *front = FrontNode {
        size: None,
        ..doors
    };
    project.rooms.clear();
    (project, library)
}

#[test]
fn a_wall_cabinet_has_a_top_and_no_toe_kick() {
    // 30" × 30" × 12": the box is the full 762 mm high.
    let (project, library) = variant("wall-cabinet@1", 30, 30, 12);
    assert_eq!(
        b30(&project, &library, ""),
        [
            "case/side.left: 762 × 304.8 × 18, cut 762 × 304.8",
            "case/side.right: 762 × 304.8 × 18, cut 762 × 304.8",
            "case/bottom: 726 × 304.8 × 18, cut 726 × 304.8",
            "case/top: 726 × 304.8 × 18, cut 726 × 304.8",
            // Grooved into the top and the bottom: H − 2T + 2g.
            "case/back: 745.05 × 745.05 × 12, cut 745.05 × 745.05",
            // The face, 762 mm square, in two halves less the gap.
            "front/403/door.1: 758.825 × 377.825 × 18, cut 757.825 × 376.825",
            "front/403/door.2: 758.825 × 377.825 × 18, cut 757.825 × 376.825",
        ]
    );
}

#[test]
fn a_tall_cabinet_has_a_top_and_a_toe_kick() {
    // 24" × 84" × 24": Hb = 2133.6 − 101.6 = 2032 mm.
    let (project, library) = variant("tall-cabinet@1", 24, 84, 24);
    assert_eq!(
        b30(&project, &library, "case/"),
        [
            "case/side.left: 2032 × 609.6 × 18, cut 2032 × 609.6",
            "case/side.right: 2032 × 609.6 × 18, cut 2032 × 609.6",
            "case/bottom: 573.6 × 609.6 × 18, cut 573.6 × 609.6",
            "case/top: 573.6 × 609.6 × 18, cut 573.6 × 609.6",
            "case/back: 2015.05 × 592.65 × 12, cut 2015.05 × 592.65",
        ]
    );
}

#[test]
fn what_cannot_be_evaluated_yet_is_reported_not_guessed() {
    let project: Project = read("project.json");
    let library: Library = read("library.json");
    let evaluation = evaluate(&project, &library);
    let problems: Vec<String> = evaluation
        .problems
        .iter()
        .map(|p| {
            let kind = if p.blocking { "error" } else { "note" };
            format!("{kind}: {}: {}", p.product, p.message)
        })
        .collect();
    assert_eq!(
        problems,
        [
            "error: B30: only slab fronts can be evaluated yet",
            "error: SB36: product type `sink-base@1` cannot be evaluated yet",
            "error: BC42: product type `blind-corner-base@1` cannot be evaluated yet",
            "error: W30: face frame cabinets cannot be evaluated yet",
            "error: IS48: no material chosen for `back`",
            "error: F3: product type `filler@1` cannot be evaluated yet",
            "error: V24: product type `vanity-base@1` cannot be evaluated yet",
        ]
    );
}

#[test]
fn bare_edges_are_noted() {
    let (project, library) = kitchen_with_slab_doors();
    let evaluation = evaluate(&project, &library);
    let notes: Vec<&str> = evaluation
        .problems
        .iter()
        .filter(|p| p.product == "B30")
        .map(|p| p.message.as_str())
        .collect();
    assert_eq!(
        notes,
        ["no edge band chosen for `edgeBandCase`; those edges are left bare"]
    );
}

#[test]
fn evaluation_is_deterministic() {
    let project: Project = read("project.json");
    let library: Library = read("library.json");
    assert_eq!(evaluate(&project, &library), evaluate(&project, &library));
}
