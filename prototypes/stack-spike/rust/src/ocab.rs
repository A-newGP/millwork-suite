// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading and writing the `.ocab` container, as a ZIP file or a directory.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Error, Result};
use crate::model::Project;

pub const MIMETYPE: &str = "application/vnd.opencabinet.project+zip";
const FORMAT_VERSION: &str = "0.1";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format_version: String,
}

/// Loads a project from a `.ocab` ZIP file or directory.
pub fn load(path: &Path) -> Result<Project> {
    let entries = read_entries(path)?;
    let entry = |name: &str| {
        entries
            .get(name)
            .ok_or_else(|| Error::InvalidContainer(format!("missing entry `{name}`")))
    };

    if entry("mimetype")?.as_slice() != MIMETYPE.as_bytes() {
        return Err(Error::InvalidContainer("wrong mimetype".into()));
    }
    let manifest: Manifest = serde_json::from_slice(entry("manifest.json")?)?;
    if manifest.format_version.split('.').next() != Some("0") {
        return Err(Error::InvalidContainer(format!(
            "unsupported format version {}",
            manifest.format_version
        )));
    }
    Ok(serde_json::from_slice(entry("project.json")?)?)
}

/// Saves a project as a `.ocab` ZIP file, `mimetype` first and uncompressed.
pub fn save_zip(project: &Project, path: &Path) -> Result<()> {
    let manifest = serde_json::json!({
        "formatVersion": FORMAT_VERSION,
        "generator": concat!("oc-spike ", env!("CARGO_PKG_VERSION")),
        "entries": ["project.json"],
    });

    let mut zip = ZipWriter::new(File::create(path)?);
    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    zip.write_all(MIMETYPE.as_bytes())?;

    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", deflated)?;
    serde_json::to_writer_pretty(&mut zip, &manifest)?;
    zip.start_file("project.json", deflated)?;
    serde_json::to_writer_pretty(&mut zip, project)?;
    zip.finish()?;
    Ok(())
}

fn read_entries(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    if path.is_dir() {
        return fs::read_dir(path)?
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path().is_file())
            .map(|entry| {
                Ok((
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path())?,
                ))
            })
            .collect();
    }

    let mut archive = ZipArchive::new(File::open(path)?)?;
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i)?;
            let mut bytes = Vec::with_capacity(usize::try_from(file.size()).unwrap_or(0));
            file.read_to_end(&mut bytes)?;
            Ok((file.name()?.into_owned(), bytes))
        })
        .collect()
}
