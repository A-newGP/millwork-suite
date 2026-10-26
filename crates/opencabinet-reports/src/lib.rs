// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reports made from a project's evaluation (RFC-0002's reports layer):
//! the cut list as rows of text in the user's units, as CSV and as PDF, and
//! the parts as DXF.

mod csv;
mod cut_list;
mod parts_dxf;
mod pdf;

pub use csv::{Separator, to_csv};
pub use cut_list::{CutList, CutRow, cut_list};
pub use parts_dxf::{ALL_PARTS, DxfFile, parts_dxf};
pub use pdf::{Fonts, Paper, cut_list_pdf};
