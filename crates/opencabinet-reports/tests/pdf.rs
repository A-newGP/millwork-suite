// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The cut list PDF of the sample kitchen. CI also checks the file with
//! independent readers (see `examples/cut_list_pdf.rs`).

use std::path::PathBuf;

use opencabinet_model::{FrontStyle, Length, Library, Project, Units};
use opencabinet_reports::{CutList, Fonts, Paper, cut_list, cut_list_pdf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fonts() -> Fonts {
    let dir = root().join("assets/fonts/noto-sans");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    Fonts::new(read("NotoSans-Regular.ttf"), read("NotoSans-Bold.ttf")).unwrap()
}

fn kitchen_list() -> CutList {
    let dir = root().join("fixtures/kitchen.ocab");
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let mut project: Project = serde_json::from_str(&read("project.json")).unwrap();
    let library: Library = serde_json::from_str(&read("library.json")).unwrap();
    let front = project.products[0].front.as_mut().unwrap();
    front.children[1].insert.as_mut().unwrap().style = Some(FrontStyle::Slab);
    cut_list(
        &project,
        &library,
        Units::Imperial,
        Length::from_inch_fraction(1, 32).unwrap(),
    )
}

#[test]
fn the_pdf_is_written_and_identical_every_time() {
    let fonts = fonts();
    let list = kitchen_list();
    let pdf = cut_list_pdf(
        &list,
        "Sample kitchen",
        "OpenCabinet test",
        Paper::Letter,
        &fonts,
    )
    .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let again = cut_list_pdf(
        &list,
        "Sample kitchen",
        "OpenCabinet test",
        Paper::Letter,
        &fonts,
    )
    .unwrap();
    assert_eq!(pdf, again);
}

#[test]
fn a_long_list_continues_on_more_pages() {
    let fonts = fonts();
    let mut list = kitchen_list();
    let rows = list.rows.clone();
    for _ in 0..10 {
        list.rows.extend(rows.iter().cloned());
    }
    let short = cut_list_pdf(&kitchen_list(), "Kitchen", "test", Paper::A4, &fonts).unwrap();
    let long = cut_list_pdf(&list, "Kitchen", "test", Paper::A4, &fonts).unwrap();
    assert!(long.len() > short.len());
}

#[test]
fn not_a_font_is_an_error() {
    assert!(Fonts::new(b"not a font".to_vec(), b"no".to_vec()).is_err());
}
