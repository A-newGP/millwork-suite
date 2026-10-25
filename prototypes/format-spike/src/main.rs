// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use oc_format::validate::{Severity, validate};
use oc_format::{Document, generate};

const USAGE: &str = "\
usage: oc-format check <project.ocab>
       oc-format convert <in.ocab> <out.ocab> [--dir]
       oc-format bench <products>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["check", path] => check(Path::new(path)),
        ["convert", from, to] => convert(Path::new(from), Path::new(to), false),
        ["convert", from, to, "--dir"] => convert(Path::new(from), Path::new(to), true),
        ["bench", products] => match products.parse() {
            Ok(n) => bench(n),
            Err(_) => Err(format!("not a number: {products}").into()),
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

type CliResult = Result<ExitCode, Box<dyn std::error::Error>>;

fn check(path: &Path) -> CliResult {
    let document = Document::load(path)?;
    let project = &document.project;
    println!(
        "{}: format {}, {} rooms, {} products, {} attachments",
        project.name,
        document.format_version,
        project.rooms.len(),
        project.products.len(),
        document.attachments.len()
    );
    for warning in &document.warnings {
        println!("warning: container: {warning}");
    }
    let issues = validate(&document);
    for issue in &issues {
        println!("{issue}");
    }
    let failed = issues.iter().any(|i| i.severity == Severity::Error);
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn convert(from: &Path, to: &Path, dir: bool) -> CliResult {
    let document = Document::load(from)?;
    if dir {
        document.save_dir(to)?;
    } else {
        document.save_zip(to)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints a Markdown table row: sizes and median save and load times.
fn bench(products: usize) -> CliResult {
    let document = generate::project(products);
    let root = std::env::temp_dir().join(format!("oc-format-bench-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let zip = root.join("bench.ocab");

    let save = median(|| document.save_zip(&zip))?;
    let load = median(|| Document::load(&zip).map(drop))?;
    let zip_size = std::fs::metadata(&zip)?.len();
    let json_size: usize = document.entries()?.iter().map(|(_, b)| b.len()).sum();
    std::fs::remove_dir_all(&root)?;

    println!("| Products | ZIP | Unpacked | Save (median of 5) | Load (median of 5) |");
    println!("| --- | --- | --- | --- | --- |");
    println!(
        "| {products} | {} KB | {} KB | {:.1} ms | {:.1} ms |",
        zip_size / 1024,
        json_size / 1024,
        save.as_secs_f64() * 1000.0,
        load.as_secs_f64() * 1000.0
    );
    Ok(ExitCode::SUCCESS)
}

fn median(mut run: impl FnMut() -> oc_format::Result<()>) -> oc_format::Result<Duration> {
    let mut times = Vec::with_capacity(5);
    for _ in 0..5 {
        let start = Instant::now();
        run()?;
        times.push(start.elapsed());
    }
    times.sort();
    Ok(times[2])
}
