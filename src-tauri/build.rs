// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

fn main() {
    // The commit `scripts/install.sh` builds from, shown next to the version.
    println!("cargo:rerun-if-env-changed=QS_COMMIT");
    tauri_build::build()
}
