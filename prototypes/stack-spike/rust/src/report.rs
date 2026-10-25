// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! CSV reports with the exact layout defined in the spike README.

use std::fmt::Write;

use crate::eval::Evaluation;

const DENOMINATOR: i64 = 32;

pub fn cut_list(evaluation: &Evaluation<'_>) -> String {
    let mut csv = String::from(
        "product,part,material,qty,length,width,thickness,length_nm,width_nm,thickness_nm,\
         cut_length_nm,cut_width_nm,band_x0,band_x1,band_y0,band_y1\n",
    );
    for p in &evaluation.parts {
        let inches = |l: crate::length::Length| l.to_fractional_inches(DENOMINATOR);
        let [x0, x1, y0, y1] = p.edges.map(|e| e.map_or("", |b| b.name.as_str()));
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{x0},{x1},{y0},{y1}",
            p.product,
            p.key,
            p.material.name,
            p.quantity,
            inches(p.length),
            inches(p.width),
            inches(p.thickness()),
            p.length,
            p.width,
            p.thickness(),
            p.cut_length(),
            p.cut_width(),
        )
        .expect("writing to a String cannot fail");
    }
    csv
}

pub fn operations(evaluation: &Evaluation<'_>) -> String {
    let mut csv = String::from("product,part,face,kind,x_nm,y_nm,diameter_nm,depth_nm\n");
    for h in &evaluation.holes {
        writeln!(
            csv,
            "{},{},B,hole,{},{},{},{}",
            h.product, h.part, h.x, h.y, h.diameter, h.depth
        )
        .expect("writing to a String cannot fail");
    }
    csv
}
