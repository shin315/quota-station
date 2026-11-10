// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The `quota-station-cli` binary ([`quota_station_cli::run`]).

fn main() {
    let Some(args) = quota_station_cli::text_args(std::env::args_os().skip(1)) else {
        eprintln!("quota-station: an argument is not valid UTF-8");
        std::process::exit(2)
    };
    let code = quota_station_cli::run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code)
}
