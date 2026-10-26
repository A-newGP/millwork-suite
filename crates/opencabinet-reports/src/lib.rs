// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reports made from a project's evaluation (RFC-0002's reports layer):
//! the cut list as rows of text in the user's units, as CSV and as PDF.

mod csv;
mod cut_list;
mod pdf;

pub use csv::{Separator, to_csv};
pub use cut_list::{CutList, CutRow, cut_list};
pub use pdf::{Fonts, Paper, cut_list_pdf};
