// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The shared helpers read the fixtures of the checkout whose tests run,
//! and refuse a test binary built from another checkout. Cargo names a
//! workspace's builds by their paths inside it, so a target folder shared
//! with another checkout (a community export built there) can hand this
//! checkout the core or the test binary built in the other one: its
//! fixtures are then the other checkout's, and once that folder is deleted
//! every fixture read fails with "fixture exists" NotFound, first in
//! `account_keys_advanced`. The checkouts below are made-up paths: no build
//! is run.

mod common;

use std::path::{Path, PathBuf};

use common::{built_elsewhere, fixture_path};

/// This crate's folder in the checkout whose tests run.
fn here() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map_or_else(|| env!("CARGO_MANIFEST_DIR").into(), PathBuf::from)
}

#[test]
fn the_fixtures_read_are_this_checkouts() {
    let path = fixture_path("claude-accounts-file.json");
    let path = path.canonicalize().expect("fixture exists");
    assert!(path.starts_with(here().canonicalize().unwrap()), "{}", path.display());
}

#[test]
fn a_core_built_in_another_checkout_is_refused_whether_or_not_it_is_still_there() {
    let here = here();
    let other = common::temp_dir("fixture-path-export");
    let core = other.join("crates/quota-station-core");
    // Deleted, as the export's folder was when the test failed.
    let why = built_elsewhere(Some(&here), &here, &core).expect("refused");
    assert!(why.contains(&core.display().to_string()), "{why}");
    assert!(why.contains(&here.display().to_string()), "{why}");
    // Still there: the fixtures would read, but they are not this checkout's.
    std::fs::create_dir_all(core.join("tests/fixtures")).unwrap();
    assert!(built_elsewhere(Some(&here), &here, &core).is_some());
    std::fs::remove_dir_all(&other).unwrap();
}

#[test]
fn a_test_binary_built_in_another_checkout_is_refused() {
    let core = Path::new("/elsewhere/export/crates/quota-station-core");
    assert!(built_elsewhere(Some(&here()), core, core).is_some());
}

#[test]
fn the_core_and_a_crate_beside_it_in_one_checkout_are_kept() {
    let core = Path::new("/checkout/crates/quota-station-core");
    assert_eq!(built_elsewhere(Some(core), core, core), None);
    assert_eq!(built_elsewhere(None, core, core), None);
    // Another crate of the checkout that includes the helpers by path.
    let other = Path::new("/checkout/tools/reader");
    assert_eq!(built_elsewhere(Some(other), other, core), None);
    assert!(built_elsewhere(Some(other), Path::new("/elsewhere/tools/reader"), core).is_some());
}
