// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The `quota-station-cli` binary ([`quota_station_cli::run`]).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = quota_station_cli::run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code)
}
