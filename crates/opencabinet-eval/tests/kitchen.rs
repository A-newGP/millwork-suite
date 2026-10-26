// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The evaluator against the sample kitchen in `fixtures/`, with sizes
//! worked out by hand from the formulas in the crate documentation.

use std::path::PathBuf;

use opencabinet_eval::{Part, evaluate};
use opencabinet_model::{FrontStyle, Library, Project};
use serde::de::DeserializeOwned;

fn read<T: DeserializeOwned>(path: &str) -> T {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/kitchen.ocab")
        .join(path);
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// `(key, length, width, thickness, cut length, cut width)` in nanometers,
/// with the key's front node id shortened to its last three digits.
fn row(part: &Part) -> (String, i64, i64, i64, i64, i64) {
    let key = part.key.replace("0199f8a0-0000-7000-8000-000000000", "");
    (
        key,
        part.length.nm(),
        part.width.nm(),
        part.thickness.nm(),
        part.cut_length.nm(),
        part.cut_width.nm(),
    )
}

/// The sample kitchen, with the B30's five-piece doors, which are not
/// evaluated yet, made slabs.
fn kitchen_with_slab_doors() -> (Project, Library) {
    let mut project: Project = read("project.json");
    let front = project.products[0].front.as_mut().unwrap();
    front.children[1].insert.as_mut().unwrap().style = Some(FrontStyle::Slab);
    (project, read("library.json"))
}

#[test]
fn the_base_cabinet_gives_the_parts_worked_out_by_hand() {
    let (project, library) = kitchen_with_slab_doors();
    let evaluation = evaluate(&project, &library);
    let b30: Vec<&Part> = evaluation
        .parts
        .iter()
        .filter(|p| p.product == project.products[0].id)
        .collect();

    // W = 30", H = 34-1/2", D = 24", T = 18 mm (melamine), back 12 mm,
    // K = 4", S = 4", G = 1/8", groove 3/8", fronts banded with 0.5 mm.
    // Hb = 34-1/2" − 4" = 774.7 mm; W − 2T = 726 mm.
    let expected: [(&str, i64, i64, i64, i64, i64); 9] = [
        (
            "case/side.left",
            774_700_000,
            609_600_000,
            18_000_000,
            774_700_000,
            609_600_000,
        ),
        (
            "case/side.right",
            774_700_000,
            609_600_000,
            18_000_000,
            774_700_000,
            609_600_000,
        ),
        (
            "case/bottom",
            726_000_000,
            609_600_000,
            18_000_000,
            726_000_000,
            609_600_000,
        ),
        (
            "case/stretcher.front",
            726_000_000,
            101_600_000,
            18_000_000,
            726_000_000,
            101_600_000,
        ),
        (
            "case/stretcher.back",
            726_000_000,
            101_600_000,
            18_000_000,
            726_000_000,
            101_600_000,
        ),
        // Hb − T + g by W − 2T + 2g.
        (
            "case/back",
            766_225_000,
            745_050_000,
            12_000_000,
            766_225_000,
            745_050_000,
        ),
        // The 6" drawer opening less the gap, grain across: W − G by 6" − G.
        (
            "front/402/drawer/front",
            758_825_000,
            149_225_000,
            18_000_000,
            757_825_000,
            148_225_000,
        ),
        // The doors share Hb − 6" = 622.3 mm, split in two halves of 381 mm.
        (
            "front/403/door.1",
            619_125_000,
            377_825_000,
            18_000_000,
            618_125_000,
            376_825_000,
        ),
        (
            "front/403/door.2",
            619_125_000,
            377_825_000,
            18_000_000,
            618_125_000,
            376_825_000,
        ),
    ];
    let found: Vec<_> = b30.iter().map(|p| row(p)).collect();
    let expected: Vec<_> = expected
        .iter()
        .map(|(k, a, b, c, d, e)| ((*k).to_owned(), *a, *b, *c, *d, *e))
        .collect();
    assert_eq!(found, expected);
    // Placed twice in the kitchen.
    assert!(b30.iter().all(|p| p.quantity == 2));
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
            "error: W30: product type `wall-cabinet@1` cannot be evaluated yet",
            "error: IS48: no material chosen for `back`",
            "error: F3: product type `filler@1` cannot be evaluated yet",
            "error: V24: product type `vanity-base@1` cannot be evaluated yet",
        ]
    );
}

#[test]
fn evaluation_is_deterministic() {
    let project: Project = read("project.json");
    let library: Library = read("library.json");
    assert_eq!(evaluate(&project, &library), evaluate(&project, &library));
}

#[test]
fn bare_edges_and_missing_drawer_boxes_are_noted() {
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
        [
            "no edge band chosen for `edgeBandCase`; those edges are left bare",
            "drawer boxes are not evaluated yet",
        ]
    );
}
