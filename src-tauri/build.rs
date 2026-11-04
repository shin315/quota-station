// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

#[path = "src/build_number.rs"]
mod build_number;

fn main() {
    // The commit `scripts/install.sh` builds from, shown next to the version.
    println!("cargo:rerun-if-env-changed=QS_COMMIT");
    // `1` in the release workflow's build, the copy GitHub Releases offers:
    // it counts as a download, not a build from source.
    println!("cargo:rerun-if-env-changed=QS_RELEASE");
    // The build number shown with it: the build script's `QS_BUILD`, else
    // counted by git in the repository this crate is in, again whenever
    // HEAD moves.
    println!("cargo:rerun-if-env-changed=QS_BUILD");
    let dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let git = build_number::git_in(&dir);
    for file in build_number::watched(&dir, &git) {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    let from_script = std::env::var("QS_BUILD").ok();
    if let Some(n) = build_number::build_number(from_script.as_deref(), &git) {
        println!("cargo:rustc-env=QS_BUILD_NUMBER={n}");
    }
    tauri_build::build()
}
