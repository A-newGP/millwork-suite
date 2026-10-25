// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The spike's acceptance tests: exact golden outputs and a lossless round trip.

use std::path::{Path, PathBuf};

use oc_spike::{eval, ocab, report};

fn spike_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fixture() -> PathBuf {
    spike_dir().join("fixtures").join("sample.ocab")
}

fn expected(name: &str) -> String {
    std::fs::read_to_string(spike_dir().join("expected").join(name)).expect("golden file exists")
}

#[test]
fn cut_list_matches_golden_output() {
    let project = ocab::load(&fixture()).unwrap();
    let evaluation = eval::evaluate(&project).unwrap();
    assert_eq!(report::cut_list(&evaluation), expected("cutlist.csv"));
}

#[test]
fn operations_match_golden_output() {
    let project = ocab::load(&fixture()).unwrap();
    let evaluation = eval::evaluate(&project).unwrap();
    assert_eq!(report::operations(&evaluation), expected("operations.csv"));
}

#[test]
fn zip_round_trip_preserves_project_and_results() {
    let original = ocab::load(&fixture()).unwrap();
    let temp = std::env::temp_dir().join(format!("oc-spike-{}.ocab", std::process::id()));
    ocab::save_zip(&original, &temp).unwrap();
    let reopened = ocab::load(&temp);
    std::fs::remove_file(&temp).ok();

    let reopened = reopened.unwrap();
    assert_eq!(original, reopened);
    assert_eq!(
        report::cut_list(&eval::evaluate(&original).unwrap()),
        report::cut_list(&eval::evaluate(&reopened).unwrap()),
    );
}

#[test]
fn rejects_unknown_major_format_version() {
    let dir = std::env::temp_dir().join(format!("oc-spike-v9-{}.ocab", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for entry in ["mimetype", "project.json"] {
        std::fs::copy(fixture().join(entry), dir.join(entry)).unwrap();
    }
    std::fs::write(dir.join("manifest.json"), r#"{"formatVersion": "9.0"}"#).unwrap();
    let result = ocab::load(&dir);
    std::fs::remove_dir_all(&dir).ok();

    assert!(matches!(
        result,
        Err(oc_spike::error::Error::InvalidContainer(_))
    ));
}
