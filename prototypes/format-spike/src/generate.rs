// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A synthetic project of any size, for measuring file size and speed.

use std::collections::BTreeMap;

use uuid::{Builder, Uuid};

use crate::container::Document;
use crate::model::{
    Extensions, FrontNode, FrontStyle, Grain, Insert, InsertKind, Library, Method, Placement,
    Point, Product, Project, Room, Settings, SheetMaterial, Size, SplitDirection, Standard, Units,
    Unknown, Wall,
};
use crate::units::Length;

/// A valid, deterministic project with `products` base cabinets, each with a
/// drawer over a pair of doors and placed on one long wall.
pub fn project(products: usize) -> Document {
    let mut ids = Ids(0);
    let standard = ids.next();
    let material = ids.next();
    let wall_id = ids.next();
    let width = Length::inches(30, 1);

    let mut placements = Vec::with_capacity(products);
    let mut product_list = Vec::with_capacity(products);
    for i in 0..products {
        let product = Product {
            id: ids.next(),
            label: format!("B{}", i + 1),
            product_type: "base-cabinet@1".into(),
            standard,
            width,
            height: Length::inches(69, 2),
            depth: Length::inches(24, 1),
            quantity: None,
            parameters: BTreeMap::new(),
            materials: BTreeMap::from([("case".into(), material)]),
            front: Some(FrontNode {
                id: ids.next(),
                size: None,
                split: Some(SplitDirection::Horizontal),
                children: vec![
                    leaf(
                        &mut ids,
                        Size::Length(Length::inches(6, 1)),
                        InsertKind::Drawer,
                        None,
                    ),
                    leaf(&mut ids, Size::Share(1), InsertKind::Doors, Some(2)),
                ],
                insert: None,
                unknown: Unknown::new(),
            }),
            overrides: Vec::new(),
            extensions: Extensions::new(),
            unknown: Unknown::new(),
        };
        placements.push(Placement {
            id: ids.next(),
            product: product.id,
            wall: Some(wall_id),
            offset: Some(Length(width.0 * i64::try_from(i).expect("small"))),
            elevation: Some(Length(0)),
            position: None,
            rotation: None,
            unknown: Unknown::new(),
        });
        product_list.push(product);
    }

    let wall_length = Length(width.0 * i64::try_from(products.max(1)).expect("small"));
    let project = Project {
        id: ids.next(),
        name: format!("Synthetic project with {products} cabinets"),
        notes: String::new(),
        settings: Settings {
            units: Units::Imperial,
            precision: Length::inches(1, 32),
            default_standard: standard,
            unknown: Unknown::new(),
        },
        rooms: vec![Room {
            id: ids.next(),
            name: "Room".into(),
            walls: vec![Wall {
                id: wall_id,
                name: "Wall".into(),
                start: Point {
                    x: Length(0),
                    y: Length(0),
                },
                end: Point {
                    x: wall_length,
                    y: Length(0),
                },
                height: Length::inches(96, 1),
                thickness: Length::inches(9, 2),
                openings: Vec::new(),
                unknown: Unknown::new(),
            }],
            placements,
            unknown: Unknown::new(),
        }],
        products: product_list,
        revisions: Vec::new(),
        extensions: Extensions::new(),
        unknown: Unknown::new(),
    };
    let library = Library {
        materials: vec![SheetMaterial {
            id: material,
            version: 1,
            name: "Plywood 3/4\"".into(),
            nominal_thickness: Length::inches(3, 4),
            actual_thickness: Length::mm(18),
            length: Length::inches(97, 1),
            width: Length::inches(49, 1),
            grain: Grain::Length,
            unknown: Unknown::new(),
        }],
        standards: vec![Standard {
            id: standard,
            version: 1,
            name: "Frameless".into(),
            derived_from: None,
            method: Method::Frameless,
            rules: BTreeMap::new(),
            unknown: Unknown::new(),
        }],
        ..Library::default()
    };
    Document::new(project, library)
}

fn leaf(ids: &mut Ids, size: Size, kind: InsertKind, count: Option<u8>) -> FrontNode {
    FrontNode {
        id: ids.next(),
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
            unknown: Unknown::new(),
        }),
        unknown: Unknown::new(),
    }
}

/// UUIDv7 values from a fixed time and a counter, so output is reproducible.
struct Ids(u128);

impl Ids {
    fn next(&mut self) -> Uuid {
        self.0 += 1;
        let counter: [u8; 16] = self.0.to_be_bytes();
        let mut tail = [0; 10];
        tail.copy_from_slice(&counter[6..]);
        Builder::from_unix_timestamp_millis(1_761_350_400_000, &tail).into_uuid()
    }
}
