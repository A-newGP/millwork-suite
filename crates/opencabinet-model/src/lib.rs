// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The data model shared by every OpenCabinet module.
//!
//! This crate has no I/O: it defines values and the rules they obey. It
//! starts with the quantities every other type is built from:
//!
//! - [`Length`]: an exact length in integer nanometers (ADR-0009);
//! - [`Angle`]: an exact angle in integer microdegrees (RFC-0001 §2);
//! - [`Id`]: the stable identity of an authored entity (RFC-0001 §1).

mod angle;
mod id;
mod length;

pub use angle::Angle;
pub use id::{Id, ParseIdError};
pub use length::{Length, ParseLengthError, Shown, Units};
