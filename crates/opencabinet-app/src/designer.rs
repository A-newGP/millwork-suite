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
    EdgeBand, FrontNode, FrontStyle, Grain, Hardware, Id, Insert, InsertKind, Length, Library,
    Method, Product, Project, Settings, SheetMaterial, Size, SplitDirection, Standard, Units,
};
use opencabinet_reports::{
    CutList, Fonts, Paper, Separator, cut_list, cut_list_pdf, parts_dxf, to_csv,
};

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

    /// The kind of cabinet being edited, if the window offers it.
    #[must_use]
    pub fn kind(&self) -> Option<CabinetKind> {
        CabinetKind::of(&self.product().product_type)
    }

    /// Makes the cabinet another kind, with that kind's usual height, depth
    /// and front, and a label that starts with its letter. The width, the
    /// materials and the style of the fronts stay.
    pub fn set_kind(&mut self, kind: CabinetKind) {
        if self.kind() == Some(kind) {
            return;
        }
        let five_piece = self.five_piece();
        let product = &mut self.document.project.products[0];
        let (height, depth) = kind.usual_height_and_depth();
        product.product_type = kind.product_type().into();
        product.height = height;
        product.depth = depth;
        product.front = Some(kind.usual_front());
        let number: String = product
            .label
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .collect();
        product.label = format!("{}{number}", kind.label_letter());
        // The new front keeps the style the fronts had.
        self.set_five_piece(five_piece);
        self.modified = true;
    }

    /// Whether the cabinet's fronts are five-piece rather than slabs.
    #[must_use]
    pub fn five_piece(&self) -> bool {
        let mut five_piece = false;
        if let Some(front) = &self.product().front {
            visit(front, &mut |insert| {
                five_piece |= insert.style == Some(FrontStyle::FivePiece);
            });
        }
        five_piece
    }

    /// Makes every door and drawer front of the cabinet five-piece, or slab.
    pub fn set_five_piece(&mut self, five_piece: bool) {
        if self.five_piece() == five_piece {
            return;
        }
        let style = if five_piece {
            FrontStyle::FivePiece
        } else {
            FrontStyle::Slab
        };
        if let Some(front) = &mut self.document.project.products[0].front {
            visit_mut(front, &mut |insert| {
                if matches!(
                    insert.kind,
                    InsertKind::Doors | InsertKind::Drawer | InsertKind::FalseFront
                ) {
                    insert.style = Some(style.clone());
                }
            });
        }
        self.modified = true;
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

    /// Writes the cut list as a PDF to print, on the paper usual for the
    /// project's units.
    ///
    /// # Errors
    /// A message for the user when the PDF cannot be made or written.
    pub fn export_cut_list_pdf(&self, path: &Path, fonts: &Fonts) -> Result<(), String> {
        let title = &self.document.project.name;
        let paper = Paper::for_units(self.units());
        let pdf = cut_list_pdf(&self.cut_list(), title, NAME_AND_VERSION, paper, fonts)?;
        std::fs::write(path, pdf).map_err(|e| format!("Cannot export the cut list: {e}"))
    }

    /// Writes a DXF of every part, and one with all of them, into
    /// `folder`, replacing files of the same name; gives how many parts.
    ///
    /// # Errors
    /// A message for the user when a file cannot be written.
    pub fn export_parts_dxf(&self, folder: &Path) -> Result<usize, String> {
        let document = &self.document;
        let files = parts_dxf(&document.project, &document.library)?;
        let fail = |e: std::io::Error| format!("Cannot export the parts as DXF: {e}");
        std::fs::create_dir_all(folder).map_err(fail)?;
        for file in &files {
            std::fs::write(folder.join(&file.name), &file.contents).map_err(fail)?;
        }
        Ok(files.len() - 1)
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

/// Calls `f` with every insert of a front tree.
fn visit(node: &FrontNode, f: &mut impl FnMut(&Insert)) {
    if let Some(insert) = &node.insert {
        f(insert);
    }
    for child in &node.children {
        visit(child, f);
    }
}

/// Calls `f` with every insert of a front tree, to change it.
fn visit_mut(node: &mut FrontNode, f: &mut impl FnMut(&mut Insert)) {
    if let Some(insert) = &mut node.insert {
        f(insert);
    }
    for child in &mut node.children {
        visit_mut(child, f);
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

/// 3/4" plywood for the case and fronts, 1/4" plywood for the back and
/// drawer bottoms, 1/2" plywood for drawer boxes, a 0.5 mm edge band, a cup
/// hinge and a frameless standard, in that order.
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
    // The drawer slides the shop stocks, 12" to 22" by 2".
    let slides = [12, 14, 16, 18, 20, 22].map(Length::from_inches);
    let hinge = Hardware {
        id: Id::new(),
        version: 1,
        category: "hinge".into(),
        name: "Cup hinge 35 mm".into(),
        dimensions: [
            ("cupDiameter".to_owned(), Length::from_mm(35)),
            ("cupDepth".to_owned(), Length::from_mm(13)),
        ]
        .into(),
        unknown: BTreeMap::default(),
    };
    let rules = serde_json::json!({
        "back": {
            "construction": "grooved",
            "grooveDepth": inches(3, 8),
            "setback": inches(1, 4),
        },
        "case": { "stretcherWidth": Length::from_inches(4) },
        "drawers": {
            "sideClearance": inches(1, 2),
            "verticalClearance": inches(3, 2),
            "rearClearance": Length::from_inches(1),
            "slideLengths": slides,
            "bottomGrooveDepth": inches(1, 4),
            "bottomGrooveSetback": inches(1, 2),
        },
        "fivePiece": {
            "stileWidth": inches(9, 4),
            "railWidth": inches(9, 4),
            "jointDepth": inches(3, 8),
            "panelGrooveDepth": inches(3, 8),
            "panelClearance": inches(1, 8),
        },
        "fronts": { "overlay": "full", "gap": inches(1, 8) },
        "shelves": {
            "sideClearance": inches(1, 16),
            "frontSetback": inches(1, 4),
            "backAllowance": inches(3, 4),
        },
        "toeKick": { "height": Length::from_inches(4) },
        "systemHoles": {
            "frontSetback": Length::from_mm(37),
            "rearSetback": Length::from_mm(37),
            "spacing": Length::from_mm(32),
            "diameter": Length::from_mm(5),
            "depth": Length::from_mm(12),
            "endClearance": Length::from_inches(2),
        },
        "hinges": {
            "hardware": hinge.id,
            "boringDistance": Length::from_mm(4),
            "endDistance": Length::from_inches(3),
            "counts": [
                { "upTo": Length::from_inches(40), "count": 2 },
                { "upTo": Length::from_inches(60), "count": 3 },
                { "upTo": Length::from_inches(80), "count": 4 },
            ],
        },
    });
    Library {
        materials: vec![
            sheet("Plywood 3/4\"", inches(3, 4), Length::from_mm(18)),
            sheet("Plywood 1/4\"", inches(1, 4), Length::from_mm(6)),
            sheet("Plywood 1/2\"", inches(1, 2), Length::from_mm(12)),
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
        hardware: vec![hinge],
        unknown: BTreeMap::default(),
    }
}

/// A 30" × 34-1/2" × 24" base cabinet from the starter library.
fn base_cabinet(library: &Library) -> Product {
    let (plywood, thin, box_sides, band) = (
        library.materials[0].id,
        library.materials[1].id,
        library.materials[2].id,
        library.edge_bands[0].id,
    );
    let (height, depth) = CabinetKind::Base.usual_height_and_depth();
    Product {
        id: Id::new(),
        label: "B1".into(),
        product_type: CabinetKind::Base.product_type().into(),
        standard: library.standards[0].id,
        width: Length::from_inches(30),
        height,
        depth,
        quantity: None,
        parameters: BTreeMap::default(),
        materials: [
            ("case", plywood),
            ("back", thin),
            ("doors", plywood),
            ("drawerBox", box_sides),
            ("drawerBottom", thin),
            ("frontPanels", thin),
            ("edgeBandCase", band),
            ("edgeBandFronts", band),
        ]
        .into_iter()
        .map(|(role, id)| (role.to_owned(), id))
        .collect(),
        front: Some(CabinetKind::Base.usual_front()),
        overrides: Vec::new(),
        extensions: BTreeMap::default(),
        unknown: BTreeMap::default(),
    }
}

/// The kinds of cabinet the window offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CabinetKind {
    /// Stands on the floor, under the counter.
    Base,
    /// Hangs on the wall.
    Wall,
    /// Floor to near the ceiling: pantries and oven towers.
    Tall,
}

impl CabinetKind {
    /// Every kind, in the order the window lists them.
    pub const ALL: [Self; 3] = [Self::Base, Self::Wall, Self::Tall];

    /// The name the window shows.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Base => "Base",
            Self::Wall => "Wall",
            Self::Tall => "Tall",
        }
    }

    fn product_type(self) -> &'static str {
        match self {
            Self::Base => "base-cabinet@1",
            Self::Wall => "wall-cabinet@1",
            Self::Tall => "tall-cabinet@1",
        }
    }

    fn of(product_type: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|k| k.product_type() == product_type)
    }

    /// The first letter of the shop's usual labels: B30, W30, T24.
    fn label_letter(self) -> char {
        self.name().chars().next().expect("names are not empty")
    }

    /// 34-1/2" × 24" for a base cabinet, 30" × 12" for a wall cabinet,
    /// 84" × 24" for a tall one.
    fn usual_height_and_depth(self) -> (Length, Length) {
        match self {
            Self::Base => (inches(69, 2), Length::from_inches(24)),
            Self::Wall => (Length::from_inches(30), Length::from_inches(12)),
            Self::Tall => (Length::from_inches(84), Length::from_inches(24)),
        }
    }

    /// A drawer over a pair of doors for a base cabinet; a pair of doors
    /// with two shelves for a wall cabinet, or four for a tall one.
    fn usual_front(self) -> FrontNode {
        let leaf = |size, kind, count, shelves| FrontNode {
            id: Id::new(),
            size,
            split: None,
            children: Vec::new(),
            insert: Some(Insert {
                kind,
                style: Some(FrontStyle::Slab),
                count,
                hinge_side: None,
                shelves,
                adjustments: Vec::new(),
                unknown: BTreeMap::default(),
            }),
            unknown: BTreeMap::default(),
        };
        match self {
            Self::Base => FrontNode {
                id: Id::new(),
                size: None,
                split: Some(SplitDirection::Horizontal),
                children: vec![
                    leaf(
                        Some(Size::Length(Length::from_inches(6))),
                        InsertKind::Drawer,
                        None,
                        None,
                    ),
                    leaf(Some(Size::Share(1)), InsertKind::Doors, Some(2), None),
                ],
                insert: None,
                unknown: BTreeMap::default(),
            },
            Self::Wall => leaf(None, InsertKind::Doors, Some(2), Some(2)),
            Self::Tall => leaf(None, InsertKind::Doors, Some(2), Some(4)),
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
        assert_eq!(list.problems, Vec::<String>::new());
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
                "Drawer box, left side",
                "Drawer box, right side",
                "Drawer box, front",
                "Drawer box, back",
                "Drawer box, bottom",
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
        assert_eq!(rows[12].edges, "4 edges");
    }

    #[test]
    fn choosing_a_kind_gives_its_usual_sizes_and_front() {
        let mut designer = Designer::default();
        designer.set_kind(CabinetKind::Wall);
        assert_eq!(designer.label(), "W1");
        assert_eq!(designer.reading(Dimension::Height), "= 30\"");
        assert_eq!(designer.reading(Dimension::Depth), "= 12\"");
        let list = designer.cut_list();
        assert_eq!(list.problems, Vec::<String>::new());
        let names: Vec<&str> = list.rows.iter().map(|r| r.part.as_str()).collect();
        assert!(
            names.contains(&"Top") && names.contains(&"Shelf 2"),
            "{names:?}"
        );

        designer.set_kind(CabinetKind::Tall);
        assert_eq!(designer.label(), "T1");
        assert_eq!(designer.reading(Dimension::Height), "= 84\"");
        assert_eq!(
            designer
                .cut_list()
                .rows
                .iter()
                .filter(|r| r.part.starts_with("Shelf"))
                .count(),
            4
        );
        assert!(designer.modified());
    }

    #[test]
    fn five_piece_fronts_replace_slab_doors_and_drawer_fronts() {
        let mut designer = Designer::default();
        assert!(!designer.five_piece());
        designer.set_five_piece(true);
        assert!(designer.five_piece());
        let list = designer.cut_list();
        assert_eq!(list.problems, Vec::<String>::new());
        let names: Vec<&str> = list.rows.iter().map(|r| r.part.as_str()).collect();
        for name in [
            "Drawer front, top rail",
            "Door 1, left stile",
            "Door 2, panel",
        ] {
            assert!(names.contains(&name), "{name} in {names:?}");
        }
        assert!(!names.contains(&"Door 1"));
        designer.set_five_piece(false);
        assert!(designer.cut_list().rows.iter().any(|r| r.part == "Door 1"));
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
