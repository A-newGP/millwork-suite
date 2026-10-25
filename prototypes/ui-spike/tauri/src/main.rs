// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#![windows_subsystem = "windows"]

use std::sync::Mutex;

use oc_ui_core::length::Length;
use oc_ui_core::{Params, Session, View};
use serde::Serialize;
use tauri::State;

/// The initial form values, formatted the way a user types them.
#[derive(Serialize)]
struct Initial {
    width: String,
    height: String,
    depth: String,
    doors: u32,
    view: View,
}

#[tauri::command]
fn initial(session: State<'_, Mutex<Session>>) -> Result<Initial, String> {
    let session = session.lock().map_err(|e| e.to_string())?;
    let params = session.params();
    let inches = |l: Length| {
        l.to_fractional_inches(32)
            .to_string()
            .trim_end_matches('"')
            .to_owned()
    };
    Ok(Initial {
        width: inches(params.width),
        height: inches(params.height),
        depth: inches(params.depth),
        doors: params.doors,
        view: session.view().map_err(|e| e.to_string())?,
    })
}

/// Parses the form in Rust, so every front end shares the same rules.
#[tauri::command]
fn apply(
    session: State<'_, Mutex<Session>>,
    width: &str,
    height: &str,
    depth: &str,
    doors: u32,
) -> Result<View, String> {
    let parse = |label: &str, text: &str| {
        Length::parse_inches(text).ok_or_else(|| format!("{label}: cannot read \"{text}\""))
    };
    let params = Params {
        width: parse("Width", width)?,
        height: parse("Height", height)?,
        depth: parse("Depth", depth)?,
        doors,
    };
    session
        .lock()
        .map_err(|e| e.to_string())?
        .apply(params)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn first_frame() {
    oc_ui_core::first_frame_presented();
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(Session::new()))
        .invoke_handler(tauri::generate_handler![initial, apply, first_frame])
        .run(tauri::generate_context!())
        .expect("Tauri application runs");
}
