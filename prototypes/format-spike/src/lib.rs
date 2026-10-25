// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Throwaway prototype of the `.ocab` project format from RFC-0001.
//!
//! It answers one question for the phase 1 exit gate: can a project be saved
//! and reopened in our own format without losing anything, including data
//! written by newer versions or by plugins? It does not evaluate products.

pub mod container;
pub mod error;
pub mod generate;
pub mod model;
pub mod units;
pub mod validate;

pub use container::{Document, FormatVersion};
pub use error::{Error, Result};
