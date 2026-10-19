//! The sessions `codex resume` can continue, counted by the provider each
//! was made with. Codex files each session under the provider id in its
//! first line (`session_meta`), and `codex resume` lists only the ones of
//! the provider in use. Only that id is looked for; nothing else in a
//! session is kept.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

/// The first line is read up to this much; the id comes early in it.
const HEAD: u64 = 256 * 1024;
/// At most this many sessions are looked at.
const MAX_FILES: usize = 20_000;

/// Sessions by provider id; `""` for one whose id could not be read.
pub fn count(codex_home: &Path) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    let mut seen = 0usize;
    let mut stack = vec![codex_home.join("sessions")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let Ok(kind) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            if kind.is_dir() {
                stack.push(e.path());
            } else if kind.is_file() && name.starts_with("rollout-") && name.ends_with(".jsonl") {
                if seen >= MAX_FILES {
                    return out;
                }
                seen += 1;
                *out.entry(provider(&e.path()).unwrap_or_default()).or_insert(0) += 1;
            }
        }
    }
    out
}

/// The `model_provider` of the session's first line.
pub fn provider(file: &Path) -> Option<String> {
    let mut head = Vec::new();
    std::fs::File::open(file).ok()?.take(HEAD).read_to_end(&mut head).ok()?;
    let line = head.split(|b| *b == b'\n').next()?;
    let text = String::from_utf8_lossy(line);
    let at = text.find("\"model_provider\"")?;
    let rest = text[at + "\"model_provider\"".len()..].trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
