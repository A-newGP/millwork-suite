// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::hint::black_box;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use oc_spike::error::Result;
use oc_spike::{eval, ocab, report};

const USAGE: &str = "usage: oc-spike evaluate <input> <output-dir>
       oc-spike roundtrip <input> <temp.ocab>
       oc-spike bench <input> <iterations>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["evaluate", input, out] => evaluate(Path::new(input), Path::new(out)),
        ["roundtrip", input, temp] => roundtrip(Path::new(input), Path::new(temp)),
        ["bench", input, iterations] => match iterations.parse() {
            Ok(n) if n > 0 => bench(Path::new(input), n),
            _ => return usage(),
        },
        _ => return usage(),
    };
    match outcome {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(1)
}

fn evaluate(input: &Path, out: &Path) -> Result<ExitCode> {
    let project = ocab::load(input)?;
    let evaluation = eval::evaluate(&project)?;
    std::fs::create_dir_all(out)?;
    std::fs::write(out.join("cutlist.csv"), report::cut_list(&evaluation))?;
    std::fs::write(out.join("operations.csv"), report::operations(&evaluation))?;
    Ok(ExitCode::SUCCESS)
}

fn roundtrip(input: &Path, temp: &Path) -> Result<ExitCode> {
    let original = ocab::load(input)?;
    ocab::save_zip(&original, temp)?;
    let reopened = ocab::load(temp)?;

    let same = original == reopened && eval::evaluate(&original)? == eval::evaluate(&reopened)?;
    if same {
        println!("round trip OK");
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!("round trip mismatch");
        Ok(ExitCode::from(3))
    }
}

fn bench(input: &Path, iterations: u32) -> Result<ExitCode> {
    let project = ocab::load(input)?;
    let start = Instant::now();
    for _ in 0..iterations {
        let evaluation = eval::evaluate(black_box(&project))?;
        black_box(evaluation);
    }
    let per_iteration = start.elapsed().as_nanos() / u128::from(iterations);
    println!("{per_iteration} ns per evaluation ({iterations} iterations)");
    Ok(ExitCode::SUCCESS)
}
