//! "Giá API tương đương": what the tokens would cost at the providers' API
//! prices, only to compare with the subscriptions the owner pays for. This
//! is the one table of prices in the app; nothing fetches them, they change
//! with the app.
//!
//! USD per million tokens, standard tier, short prompts. Claude's cache
//! write is the five-minute one (1.25 × input). OpenAI bills no cache write.

/// Where the prices come from, and when they were copied.
pub const SOURCE: &str = "anthropic.com/pricing · openai.com/api/pricing";
pub const AS_OF: &str = "2026-10-19";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub cache_write: f64,
    pub cache_read: f64,
    pub output: f64,
}

const fn claude(input: f64, cache_read: f64, output: f64) -> Price {
    Price { input, cache_write: input * 1.25, cache_read, output }
}

const fn openai(input: f64, cache_read: f64, output: f64) -> Price {
    Price { input, cache_write: 0.0, cache_read, output }
}

/// By model id, as the logs write it (a date after it is allowed).
const TABLE: &[(&str, Price)] = &[
    ("claude-fable-5-1", claude(10.0, 0.25, 50.0)),
    ("claude-fable-5", claude(10.0, 1.0, 50.0)),
    ("claude-opus-5-5", claude(4.0, 0.20, 20.0)),
    ("claude-opus-5", claude(5.0, 0.50, 25.0)),
    ("claude-opus-4-8", claude(5.0, 0.50, 25.0)),
    ("claude-opus-4-7", claude(5.0, 0.50, 25.0)),
    ("claude-opus-4-6", claude(5.0, 0.50, 25.0)),
    ("claude-opus-4-5", claude(5.0, 0.50, 25.0)),
    ("claude-opus-4-1", claude(15.0, 1.50, 75.0)),
    ("claude-opus-4", claude(15.0, 1.50, 75.0)),
    ("claude-sonnet-5-5", claude(2.0, 0.20, 10.0)),
    ("claude-sonnet-5", claude(2.0, 0.20, 10.0)),
    ("claude-sonnet-4-6", claude(3.0, 0.30, 15.0)),
    ("claude-sonnet-4-5", claude(3.0, 0.30, 15.0)),
    ("claude-sonnet-4", claude(3.0, 0.30, 15.0)),
    ("claude-haiku-5-5", claude(0.10, 0.01, 0.50)),
    ("claude-haiku-4-5", claude(1.0, 0.10, 5.0)),
    ("claude-3-5-haiku", claude(0.80, 0.08, 4.0)),
    ("gpt-5.6-sol", openai(4.0, 0.40, 20.0)),
    ("gpt-5.6-terra", openai(2.0, 0.20, 12.0)),
    ("gpt-5.6-luna", openai(0.20, 0.02, 1.20)),
    ("gpt-5.5", openai(5.0, 0.50, 30.0)),
    ("gpt-5.4", openai(2.50, 0.25, 15.0)),
    ("gpt-5.4-mini", openai(0.75, 0.075, 4.50)),
    ("gpt-5.4-nano", openai(0.20, 0.02, 1.25)),
    ("gpt-5.3-codex", openai(1.75, 0.175, 14.0)),
    ("gpt-5.2-codex", openai(1.75, 0.175, 14.0)),
    ("gpt-5.2", openai(1.75, 0.175, 14.0)),
    ("gpt-5.1-codex-max", openai(1.25, 0.125, 10.0)),
    ("gpt-5.1-codex-mini", openai(0.25, 0.025, 2.0)),
    ("gpt-5.1-codex", openai(1.25, 0.125, 10.0)),
    ("gpt-5.1", openai(1.25, 0.125, 10.0)),
    ("gpt-5-codex", openai(1.25, 0.125, 10.0)),
    ("gpt-5-mini", openai(0.25, 0.025, 2.0)),
    ("gpt-5-nano", openai(0.05, 0.005, 0.40)),
    ("gpt-5", openai(1.25, 0.125, 10.0)),
];

/// The price of `model`, when the table has it: the id itself, or the id
/// followed by a date (`claude-sonnet-4-5-20250929`, `gpt-5-2025-08-07`).
pub fn price(model: &str) -> Option<Price> {
    let model = model.trim().to_ascii_lowercase();
    TABLE
        .iter()
        .find(|(id, _)| {
            model.strip_prefix(id).is_some_and(|rest| {
                rest.is_empty() || rest.strip_prefix(['-', '@']).is_some_and(is_date)
            })
        })
        .map(|(_, p)| *p)
}

/// `20250929` or `2025-08-07`.
fn is_date(text: &str) -> bool {
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    match text.split('-').collect::<Vec<_>>()[..] {
        [d] => digits(d, 8),
        [y, m, d] => digits(y, 4) && digits(m, 2) && digits(d, 2),
        _ => false,
    }
}

/// The cost of `tokens` at `price`, in USD.
pub fn cost(price: &Price, tokens: &super::Tokens) -> f64 {
    (tokens.input as f64 * price.input
        + tokens.cache_write as f64 * price.cache_write
        + tokens.cache_read as f64 * price.cache_read
        + tokens.output as f64 * price.output)
        / 1_000_000.0
}
