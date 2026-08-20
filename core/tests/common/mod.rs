//! Shared by the integration tests. Fixtures are read from disk here because a
//! test is a consumer of the core, and consumers are where IO lives (ADR-0002).

// Each test binary compiles this module separately, so a helper used by only
// one of them looks dead to the others. Not a suppressed warning — a false one.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

fn corpus_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("tests/fixtures");
    p
}

/// The bytes of a fixture, by path relative to `tests/fixtures`.
pub fn fixture(name: &str) -> Vec<u8> {
    let p = corpus_root().join(name);
    fs::read(&p).unwrap_or_else(|e| panic!("fixture {}: {e}", p.display()))
}

/// Every fixture in the corpus, so a property that must hold for *any* document
/// is asserted against all of them rather than against a chosen one.
pub fn every_fixture() -> Vec<(String, Vec<u8>)> {
    let root = corpus_root();
    let mut out = Vec::new();
    let mut dirs = vec![root.clone()];

    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("fixture directory should exist") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "docx") {
                let name = path
                    .strip_prefix(&root)
                    .expect("under the fixture root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((name, fs::read(&path).expect("readable fixture")));
            }
        }
    }

    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty(), "the corpus should not be empty");
    out
}
