// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Whether the custom provider answers: a `GET <base_url>/models` with no
//! key and no login, so nothing secret leaves the Mac. Any HTTP answer, a
//! 401 included, counts as answering; only no answer within five seconds,
//! three times in a row, counts as down.

use std::time::{Duration, Instant};

use serde::Serialize;

pub const TIMEOUT: Duration = Duration::from_secs(5);
pub const TRIES: u32 = 3;
const PAUSE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Answer {
    /// Answered, in this many milliseconds.
    Up { ms: u64 },
    /// No answer in [`TRIES`] tries.
    Down { detail: String },
    /// The base URL is not one the app can ask.
    BadUrl,
}

fn models_url(base: &str) -> Option<url::Url> {
    let url = url::Url::parse(&format!("{}/models", base.trim_end_matches('/'))).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(url)
}

/// Asks once per try, a second apart, until one answers.
pub fn check(base_url: &str) -> Answer {
    check_with(base_url, TIMEOUT, PAUSE)
}

pub fn check_with(base_url: &str, timeout: Duration, pause: Duration) -> Answer {
    let Some(url) = models_url(base_url) else { return Answer::BadUrl };
    let client = match reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => return Answer::Down { detail: e.to_string() },
    };
    let mut detail = String::new();
    for i in 0..TRIES {
        if i > 0 {
            std::thread::sleep(pause);
        }
        let started = Instant::now();
        match client.get(url.clone()).send() {
            Ok(_) => return Answer::Up { ms: started.elapsed().as_millis() as u64 },
            Err(e) => detail = if e.is_timeout() { "timeout".into() } else { "connect".into() },
        }
    }
    Answer::Down { detail }
}
