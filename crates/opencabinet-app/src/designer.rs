// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the window shows and edits, without any window: the open project,
//! where it is saved, and the cabinet being sized. Keeping it free of Slint
//! lets it be tested directly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opencabinet_format::Document;
use opencabinet_model::validate::{Issue, validate};
use opencabinet_model::{
    EdgeBand, FrontNode, FrontStyle, Grain, Id, Insert, InsertKind, Length, Library, Method,
    Product, Project, Settings, SheetMaterial, Size, SplitDirection, Standard, Units,
};
use opencabinet_reports::{CutList, Separator, cut_list, to_csv};

use crate::NAME_AND_VERSION;

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

/// The editing state of the window: a project and the file it lives in.
///
/// The window edits the project's first product; editing more products
/// arrives with the designer's product list.
#[derive(Clone, Debug, PartialEq)]
pub struct Designer {
    document: Document,
    path: Option<PathBuf>,
    modified: bool,
}

impl Default for Designer {
    /// A new project in inches.
    fn default() -> Self {
        Self::new(Units::Imperial)
    }
}

impl Designer {
    /// A new, unsaved project with one 30" × 34-1/2" × 24" base cabinet,
    /// shown in `units`.
    #[must_use]
    pub fn new(units: Units) -> Self {
        Self {
            document: new_document(units),
            path: None,
            modified: false,
        }
    }

    /// Opens a project file.
    ///
    /// # Errors
    /// A message for the user when the file cannot be read or has no
    /// product to show; the current project is then left unchanged.
    pub fn open(path: &Path) -> Result<Self, String> {
        let document = Document::load(path).map_err(|e| format!("Cannot open the file: {e}"))?;
        if document.project.products.is_empty() {
            return Err("The project has no products to show yet".into());
        }
        Ok(Self {
            document,
            path: Some(path.to_owned()),
            modified: false,
        })
    }

    /// Saves the project to `path`, which becomes its file.
    ///
    /// # Errors
    /// A message for the user when the file cannot be written.
    pub fn save_as(&mut self, path: &Path) -> Result<(), String> {
        self.document.generator = NAME_AND_VERSION.into();
        self.document
            .save_zip(path)
            .map_err(|e| format!("Cannot save the file: {e}"))?;
        self.path = Some(path.to_owned());
        self.modified = false;
        Ok(())
    }

    /// The file the project lives in, once it has been saved or opened.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether there are changes that are not saved.
    #[must_use]
    pub fn modified(&self) -> bool {
        self.modified
    }

    /// The window title: the file's name, `*` when modified, and the program.
    #[must_use]
    pub fn title(&self) -> String {
        let name = self
            .path
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(|| "Untitled".into(), |n| n.to_string_lossy().into_owned());
        let mark = if self.modified { "*" } else { "" };
        format!("{name}{mark} — OpenCabinet Designer")
    }

    /// Problems found in the project's authored data.
    #[must_use]
    pub fn issues(&self) -> Vec<Issue> {
        validate(&self.document.project, &self.document.library)
    }

    /// The label of the product being edited, such as "B30".
    #[must_use]
    pub fn label(&self) -> &str {
        &self.product().label
    }

    /// The units lengths are typed and shown in.
    #[must_use]
    pub fn units(&self) -> Units {
        self.document.project.settings.units
    }

    /// Changes the units lengths are shown in, with that system's usual
    /// precision. Stored lengths never change, but the preference is saved
    /// with the project.
    pub fn set_units(&mut self, units: Units) {
        let settings = &mut self.document.project.settings;
        if settings.units != units {
            settings.units = units;
            settings.precision = usual_precision(units);
            self.modified = true;
        }
    }

    /// A dimension's stored value.
    #[must_use]
    pub fn get(&self, dimension: Dimension) -> Length {
        let product = self.product();
        match dimension {
            Dimension::Width => product.width,
            Dimension::Height => product.height,
            Dimension::Depth => product.depth,
        }
    }

    /// Reads what the user typed into a dimension and stores it.
    ///
    /// # Errors
    /// A message for the user when the text is not a positive length; the
    /// stored value is then left unchanged.
    pub fn edit(&mut self, dimension: Dimension, text: &str) -> Result<(), String> {
        let length = Length::parse(text, self.units()).map_err(|e| capitalize(&e.to_string()))?;
        if length <= Length::ZERO {
            return Err("Must be greater than zero".into());
        }
        let product = &mut self.document.project.products[0];
        let field = match dimension {
            Dimension::Width => &mut product.width,
            Dimension::Height => &mut product.height,
            Dimension::Depth => &mut product.depth,
        };
        if *field != length {
            *field = length;
            self.modified = true;
        }
        Ok(())
    }

    /// A dimension as text to put in its field, in the current units.
    #[must_use]
    pub fn text(&self, dimension: Dimension) -> String {
        self.get(dimension)
            .show(self.units(), self.precision())
            .text
    }

    /// How a dimension was read, for the line under its field: `= 30-1/2"`,
    /// or `≈ 30-1/2"` when the display precision rounds it.
    #[must_use]
    pub fn reading(&self, dimension: Dimension) -> String {
        let shown = self.get(dimension).show(self.units(), self.precision());
        let sign = if shown.exact { '=' } else { '≈' };
        format!("{sign} {}", shown.text)
    }

    /// The project's cut list, in the current units.
    #[must_use]
    pub fn cut_list(&self) -> CutList {
        let document = &self.document;
        cut_list(
            &document.project,
            &document.library,
            self.units(),
            self.precision(),
        )
    }

    /// Writes the cut list as CSV, with `separator` between fields.
    ///
    /// # Errors
    /// A message for the user when the file cannot be written.
    pub fn export_cut_list(&self, path: &Path, separator: Separator) -> Result<(), String> {
        std::fs::write(path, to_csv(&self.cut_list(), separator))
            .map_err(|e| format!("Cannot export the cut list: {e}"))
    }

    /// Width over height, for drawing the front in proportion.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "a ratio for the screen"
    )]
    pub fn front_ratio(&self) -> f32 {
        let product = self.product();
        (product.width.nm() as f64 / product.height.nm() as f64) as f32
    }

    fn product(&self) -> &Product {
        &self.document.project.products[0]
    }

    /// The project's display precision, or the units' usual one when the
    /// file's is not usable.
    fn precision(&self) -> Length {
        let precision = self.document.project.settings.precision;
        if precision > Length::ZERO {
            precision
        } else {
            usual_precision(self.units())
        }
    }
}

/// 1/32" or 0.1 mm, the usual shop precisions.
fn usual_precision(units: Units) -> Length {
    match units {
        Units::Imperial => Length::from_inch_fraction(1, 32).expect("a valid fraction"),
        Units::Metric => Length::from_nm(100_000),
    }
}

/// A project with the materials, edge band and frameless standard a base
/// cabinet needs, and one base cabinet: a drawer over a pair of doors.
fn new_document(units: Units) -> Document {
    let library = starter_library();
    let product = base_cabinet(&library);
    let project = Project {
        id: Id::new(),
        name: "Untitled".into(),
        notes: String::new(),
        settings: Settings {
            units,
            precision: usual_precision(units),
            default_standard: library.standards[0].id,
            unknown: BTreeMap::default(),
        },
        rooms: Vec::new(),
        products: vec![product],
        revisions: Vec::new(),
        extensions: BTreeMap::default(),
        unknown: BTreeMap::default(),
    };
    let mut document = Document::new(project, library);
    document.generator = NAME_AND_VERSION.into();
    document
}

fn inches(numerator: i64, denominator: i64) -> Length {
    Length::from_inch_fraction(numerator, denominator).expect("a valid fraction")
}

/// 3/4" plywood for the case and fronts, 1/4" plywood for the back, a
/// 0.5 mm edge band and a frameless standard, in that order.
fn starter_library() -> Library {
    let sheet = |name: &str, nominal: Length, actual: Length| SheetMaterial {
        id: Id::new(),
        version: 1,
        name: name.into(),
        nominal_thickness: nominal,
        actual_thickness: actual,
        length: Length::from_inches(97),
        width: Length::from_inches(49),
        grain: Grain::Length,
        unknown: BTreeMap::default(),
    };
    let rules = serde_json::json!({
        "back": { "construction": "grooved", "grooveDepth": inches(3, 8) },
        "case": { "stretcherWidth": Length::from_inches(4) },
        "fronts": { "overlay": "full", "gap": inches(1, 8) },
        "toeKick": { "height": Length::from_inches(4) },
    });
    Library {
        materials: vec![
            sheet("Plywood 3/4\"", inches(3, 4), Length::from_mm(18)),
            sheet("Plywood 1/4\"", inches(1, 4), Length::from_mm(6)),
        ],
        edge_bands: vec![EdgeBand {
            id: Id::new(),
            version: 1,
            name: "Edge band 0.5 mm".into(),
            thickness: Length::from_nm(500_000),
            width: Length::from_mm(22),
            unknown: BTreeMap::default(),
        }],
        standards: vec![Standard {
            id: Id::new(),
            version: 1,
            name: "Frameless".into(),
            derived_from: None,
            method: Method::Frameless,
            rules: serde_json::from_value(rules).expect("rules are a JSON object"),
            unknown: BTreeMap::default(),
        }],
        hardware: Vec::new(),
        unknown: BTreeMap::default(),
    }
}

/// A 30" × 34-1/2" × 24" base cabinet from the starter library.
fn base_cabinet(library: &Library) -> Product {
    let (plywood, back, band) = (
        library.materials[0].id,
        library.materials[1].id,
        library.edge_bands[0].id,
    );
    let leaf = |size, kind, count| FrontNode {
        id: Id::new(),
        size: Some(size),
        split: None,
        children: Vec::new(),
        insert: Some(Insert {
            kind,
            style: Some(FrontStyle::Slab),
            count,
            hinge_side: None,
            shelves: None,
            adjustments: Vec::new(),
            unknown: BTreeMap::default(),
        }),
        unknown: BTreeMap::default(),
    };
    Product {
        id: Id::new(),
        label: "B1".into(),
        product_type: "base-cabinet@1".into(),
        standard: library.standards[0].id,
        width: Length::from_inches(30),
        height: inches(69, 2),
        depth: Length::from_inches(24),
        quantity: None,
        parameters: BTreeMap::default(),
        materials: [
            ("case", plywood),
            ("back", back),
            ("doors", plywood),
            ("edgeBandCase", band),
            ("edgeBandFronts", band),
        ]
        .into_iter()
        .map(|(role, id)| (role.to_owned(), id))
        .collect(),
        front: Some(FrontNode {
            id: Id::new(),
            size: None,
            split: Some(SplitDirection::Horizontal),
            children: vec![
                leaf(
                    Size::Length(Length::from_inches(6)),
                    InsertKind::Drawer,
                    None,
                ),
                leaf(Size::Share(1), InsertKind::Doors, Some(2)),
            ],
            insert: None,
            unknown: BTreeMap::default(),
        }),
        overrides: Vec::new(),
        extensions: BTreeMap::default(),
        unknown: BTreeMap::default(),
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

    fn scratch(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("opencabinet-app-{}-{test}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_new_project_is_valid_and_unmodified() {
        let designer = Designer::default();
        assert_eq!(designer.issues(), []);
        assert!(!designer.modified());
        assert_eq!(designer.title(), "Untitled — OpenCabinet Designer");
        assert_eq!(designer.label(), "B1");
    }

    #[test]
    fn the_cut_list_follows_edits_and_units() {
        let mut designer = Designer::default();
        let list = designer.cut_list();
        assert_eq!(
            list.problems,
            ["B1, note: drawer boxes are not evaluated yet"]
        );
        let names: Vec<&str> = list.rows.iter().map(|r| r.part.as_str()).collect();
        assert_eq!(
            names,
            [
                "Left side",
                "Right side",
                "Bottom",
                "Front stretcher",
                "Back stretcher",
                "Back",
                "Drawer front",
                "Door 1",
                "Door 2",
            ]
        );
        // The bottom spans the inside: 30" less two 18 mm sides.
        assert_eq!(list.rows[2].length, "28-19/32\"");
        assert_eq!(list.rows[2].edges, "1 edge");

        designer.edit(Dimension::Width, "36").unwrap();
        designer.set_units(Units::Metric);
        let rows = designer.cut_list().rows;
        assert_eq!(
            (rows[2].length.as_str(), rows[2].cut_length.as_str()),
            ("878.4 mm", "878.4 mm")
        );
        assert_eq!(rows[7].edges, "4 edges");
    }

    #[test]
    fn edits_store_exact_values_and_keep_the_last_good_one() {
        let mut designer = Designer::default();
        designer.edit(Dimension::Width, "30 1/2").unwrap();
        assert_eq!(designer.get(Dimension::Width), Length::from_nm(774_700_000));
        assert_eq!(designer.reading(Dimension::Width), "= 30-1/2\"");
        assert!(designer.modified());

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
    fn a_saved_project_reopens_with_its_edits_and_units() {
        let path = scratch("save").join("cabinet.ocab");
        let mut designer = Designer::default();
        designer.edit(Dimension::Width, "33").unwrap();
        designer.set_units(Units::Metric);
        designer.save_as(&path).unwrap();
        assert!(!designer.modified());
        assert_eq!(designer.title(), "cabinet.ocab — OpenCabinet Designer");

        let reopened = Designer::open(&path).unwrap();
        assert_eq!(reopened.get(Dimension::Width), Length::from_inches(33));
        assert_eq!(reopened.units(), Units::Metric);
        assert_eq!(reopened, designer);
    }

    #[test]
    fn opening_something_that_is_not_a_project_explains_why() {
        let path = scratch("open").join("notes.ocab");
        std::fs::write(&path, "not a project").unwrap();
        let error = Designer::open(&path).unwrap_err();
        assert!(error.starts_with("Cannot open the file:"), "{error}");
    }

    #[test]
    fn the_front_ratio_follows_width_and_height() {
        let mut designer = Designer::default();
        designer.edit(Dimension::Width, "69/2").unwrap();
        assert!((designer.front_ratio() - 1.0).abs() < f32::EPSILON);
    }
}
