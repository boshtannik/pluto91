//! Emits a `face_*` cfg flag for every face listed in `faces.toml`, so faces
//! that are not listed are simply not compiled into the build.
//!
//! The `src/lib.rs` gates its modules, the `Faces` enum variants and the
//! `FaceSet` table on these flags. See `faces.toml` for the list of faces.

use std::env;
use std::fs;
use std::path::Path;

/// Every face the crate knows about, in the order they appear in the `Faces`
/// enum: `(name in faces.toml, cfg flag)`.
const KNOWN: &[(&str, &str)] = &[
    ("simple_clock", "face_simple_clock"),
    ("alarm", "face_alarm"),
    ("simple_alarm", "face_simple_alarm"),
    ("timer", "face_timer"),
    ("calendar", "face_calendar"),
];

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let text = fs::read_to_string(Path::new(&dir).join("faces.toml"))
        .expect("cannot read faces.toml (in crates/pluto-faces/)");

    println!("cargo:rerun-if-changed=faces.toml");
    println!("cargo:rerun-if-changed=build.rs");

    let enabled = parse_faces(&text);

    // Validate the list: only known names, and simple_clock is the default
    // face the runtime boots into, so it can never be dropped.
    for name in &enabled {
        if !KNOWN.iter().any(|(n, _)| n == name) {
            let known: Vec<_> = KNOWN.iter().map(|(n, _)| n).collect();
            panic!("faces.toml: unknown face {name:?}; available: {known:?}");
        }
    }
    if !enabled.iter().any(|n| n == "simple_clock") {
        panic!("faces.toml: simple_clock must always be enabled (it is the default face)");
    }

    // Declare the known flags (so disabled faces do not trip the
    // `unexpected_cfgs` lint) and enable the listed ones.
    for (_, flag) in KNOWN {
        println!("cargo::rustc-check-cfg=cfg({flag})");
    }
    for name in &enabled {
        let flag = KNOWN.iter().find(|(n, _)| n == name).expect("validated").1;
        println!("cargo::rustc-cfg={flag}");
    }
}

/// A tiny parser for the `faces = ["a", "b"]` line in `faces.toml`. Comment
/// (`#`) and blank lines are ignored; any other keys are ignored too.
fn parse_faces(text: &str) -> Vec<String> {
    let mut faces = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let rest = match line.strip_prefix("faces") {
            Some(rest) => rest,
            None => continue,
        };
        let rest = rest
            .trim()
            .strip_prefix('=')
            .expect("faces.toml: expected 'faces = [ ... ]'")
            .trim()
            .strip_prefix('[')
            .expect("faces.toml: expected 'faces = [ ... ]'");
        let rest = rest.split(']').next().unwrap_or("").trim();
        for item in rest.split(',') {
            let item = item.trim().trim_matches('"').trim();
            if !item.is_empty() {
                faces.push(item.to_string());
            }
        }
    }
    faces
}
