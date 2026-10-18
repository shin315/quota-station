//! Shared helpers for the fixture tests; each test file uses a subset.
//! Every fixture is synthetic: made-up slot numbers, percentages and
//! times, generated at 2026-01-16 07:20 UTC.
#![allow(dead_code)]

use std::path::PathBuf;

use quota_station_core::axi::{self, Response};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).expect("fixture exists")
}

pub fn fixture(name: &str) -> Response {
    axi::parse(&fixture_text(name)).expect("fixture parses")
}
