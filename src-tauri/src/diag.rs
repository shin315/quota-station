// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The diagnostics bundle of the test build (plan 4.3): `diag-<date>.zip`,
//! which the owner drags into a chat after a guided session instead of
//! reporting anything himself. It holds the diagnostic info of Cài đặt ›
//! Cập nhật, `settings.json`, the app's logs, each step of the session with
//! the result of its self-checks, and the pictures the app took of its own
//! windows and tray.
//!
//! Only files picked by name go in: never a login, a key, a token, a cookie
//! or a sealed secret. Each text line goes through [`redact`] as well: an
//! email, a path outside the home folder (`C:\Users\<name>` included, see
//! [`release::scrub`]) and anything shaped like a token never shows.

use std::path::Path;

use serde_json::Value;

use crate::release;

/// A zip of stored (uncompressed) entries: the pictures are compressed
/// already, the text is small, and each entry reads as it was written.
pub struct Zip {
    out: Vec<u8>,
    central: Vec<u8>,
    entries: u16,
    time: u16,
    date: u16,
}

impl Zip {
    /// `at` is the local time every entry carries.
    pub fn new(at: time::PrimitiveDateTime) -> Zip {
        let year = u16::try_from(at.year() - 1980).unwrap_or(0).min(127);
        let date = (year << 9) | (u16::from(u8::from(at.month())) << 5) | u16::from(at.day());
        let time = (u16::from(at.hour()) << 11)
            | (u16::from(at.minute()) << 5)
            | u16::from(at.second() / 2);
        Zip { out: Vec::new(), central: Vec::new(), entries: 0, time, date }
    }

    pub fn add(&mut self, name: &str, data: &[u8]) {
        let crc = crc32(data);
        let size = u32::try_from(data.len()).unwrap_or(u32::MAX);
        let offset = u32::try_from(self.out.len()).unwrap_or(u32::MAX);
        let name_len = u16::try_from(name.len()).unwrap_or(u16::MAX);
        // Bit 11: the name is UTF-8.
        let flags: u16 = 0x0800;
        let mut local = Vec::with_capacity(30 + name.len());
        local.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        local.extend_from_slice(&20u16.to_le_bytes());
        local.extend_from_slice(&flags.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(&self.time.to_le_bytes());
        local.extend_from_slice(&self.date.to_le_bytes());
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&name_len.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name.as_bytes());
        self.out.extend_from_slice(&local);
        self.out.extend_from_slice(data);

        let c = &mut self.central;
        c.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        c.extend_from_slice(&20u16.to_le_bytes());
        c.extend_from_slice(&20u16.to_le_bytes());
        c.extend_from_slice(&flags.to_le_bytes());
        c.extend_from_slice(&0u16.to_le_bytes());
        c.extend_from_slice(&self.time.to_le_bytes());
        c.extend_from_slice(&self.date.to_le_bytes());
        c.extend_from_slice(&crc.to_le_bytes());
        c.extend_from_slice(&size.to_le_bytes());
        c.extend_from_slice(&size.to_le_bytes());
        c.extend_from_slice(&name_len.to_le_bytes());
        // Extra field, comment, disk, internal and external attributes.
        c.extend_from_slice(&[0; 12]);
        c.extend_from_slice(&offset.to_le_bytes());
        c.extend_from_slice(name.as_bytes());
        self.entries += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let at = u32::try_from(self.out.len()).unwrap_or(u32::MAX);
        let size = u32::try_from(self.central.len()).unwrap_or(u32::MAX);
        self.out.extend_from_slice(&self.central);
        self.out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        self.out.extend_from_slice(&[0; 4]);
        self.out.extend_from_slice(&self.entries.to_le_bytes());
        self.out.extend_from_slice(&self.entries.to_le_bytes());
        self.out.extend_from_slice(&size.to_le_bytes());
        self.out.extend_from_slice(&at.to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes());
        self.out
    }
}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// The CRC-32 a zip (and a PNG) checks its data with.
pub fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |c, b| CRC_TABLE[((c ^ u32::from(*b)) & 0xff) as usize] ^ (c >> 8))
}

/// An RGBA picture as a PNG, its data in stored deflate blocks: the tray
/// icon as the app drew it, a few hundred bytes.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let row = width as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for line in rgba.chunks(row.max(1)).take(height as usize) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, block) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (a, b) = raw.iter().fold((1u32, 0u32), |(a, b), x| {
        let a = (a + u32::from(*x)) % 65521;
        (a, (b + a) % 65521)
    });
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // 8 bits a channel, RGBA, deflate, no filter, no interlace.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

/// How a known token starts: Anthropic's and OpenAI's keys and OAuth
/// tokens, a JWT (ChatGPT's), Google's access and refresh tokens, GitHub's
/// and Slack's.
const TOKEN_STARTS: [&str; 12] = [
    "sk-",
    "sk_",
    "eyJ",
    "ya29.",
    "1//",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "AIza",
    "xox",
];

/// A word that may be a secret: one that starts as a known token does, or
/// a long run of letters and digits such as a key, a session id or a hash.
fn secret(part: &str) -> bool {
    if part.len() >= 12 && TOKEN_STARTS.iter().any(|s| part.starts_with(s)) {
        return true;
    }
    part.len() >= 28
        && !part.contains('/')
        && part.bytes().any(|b| b.is_ascii_digit())
        && part.bytes().any(|b| b.is_ascii_alphabetic())
}

/// One word of a line, or what stands in its place.
fn word(w: &str) -> &str {
    let bare = w.trim_start_matches(['(', '[', '"', '\'', '`', '<']);
    if w.contains('@') {
        return "[email]";
    }
    if bare.starts_with('/') || release::windows_path(bare) {
        return "[path]";
    }
    let parts = w.split(|c: char| !(c.is_ascii_alphanumeric() || "-_./+".contains(c)));
    if parts.into_iter().any(secret) {
        return "[secret]";
    }
    w
}

/// The longest line kept.
const LINE: usize = 400;

/// `text` with the home folder as `~`, and no email, path outside it, or
/// word that may be a token; each line at most [`LINE`] characters.
pub fn redact(text: &str, home: &str) -> String {
    let text = release::tilde(text, home);
    text.lines()
        .map(|line| {
            let line = line.split_whitespace().map(word).collect::<Vec<_>>().join(" ");
            line.chars().take(LINE).collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Keys whose value never goes in, whatever it holds: names of people,
/// computers and accounts, and anything secret.
const HIDDEN_KEYS: &[&str] = &[
    "machinename",
    "machine",
    // @private:begin
    "peers",
    // @private:end
    "host",
    "user",
    "username",
    "alias",
    "email",
    "label",
    "account",
    "accountname",
    "token",
    "accesstoken",
    "refreshtoken",
    "idtoken",
    "apikey",
    "secret",
    "password",
    "cookie",
    "authorization",
];

fn hidden(key: &str) -> bool {
    let key: String = key.chars().filter(|c| *c != '_' && *c != '-').collect();
    HIDDEN_KEYS.contains(&key.to_ascii_lowercase().as_str())
}

/// A JSON value with every hidden key's value replaced and every string
/// redacted.
pub fn clean(value: Value, home: &str) -> Value {
    match value {
        Value::String(s) => Value::String(redact(&s, home)),
        Value::Array(items) => Value::Array(items.into_iter().map(|v| clean(v, home)).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| {
                    let v = if !hidden(&k) || v.is_null() {
                        clean(v, home)
                    } else {
                        Value::String("[hidden]".into())
                    };
                    (k, v)
                })
                .collect(),
        ),
        other => other,
    }
}

/// JSON Lines, each line cleaned; a line that is not JSON is redacted.
fn clean_lines(text: &str, home: &str) -> String {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(v) => clean(v, home).to_string(),
            Err(_) => redact(line, home),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The most of each log that goes in: its last lines.
const LOG_LINES: usize = 400;
/// The pictures stop at this, so the zip stays under what a chat takes.
const PICTURES: usize = 24 * 1024 * 1024;

/// What a bundle is made of.
pub struct Sources<'a> {
    /// The home folder, shown as `~`.
    pub home: &'a str,
    pub at: time::PrimitiveDateTime,
    /// Cài đặt › Cập nhật's diagnostic info ([`release::diagnostics`]).
    pub diagnostics: String,
    /// The app's `settings.json`, as it is on disk.
    pub settings: Option<String>,
    /// The app's logs: a name and the file's text.
    pub logs: Vec<(String, String)>,
    /// The session's own folder: its progress, its events, each step's
    /// results, and the pictures the app took.
    pub session: Option<&'a Path>,
}

const README: &str = "\
Quota Station diagnostics (test build)

diagnostics.txt   the diagnostic info of Cài đặt › Cập nhật
settings.json     the app's settings, without names or paths outside ~
logs/             the app's logs, last lines
session/          the guided session: progress, events, each step and its self-checks
shots/            pictures the app took of its own windows and the tray corner
tray/             the tray icon as the app drew it

No login, token, key, cookie, email or Credential Manager item is in here.
";

/// The zip: a few named files from the app, each cleaned, and the session.
pub fn bundle(src: &Sources) -> Vec<u8> {
    let home = src.home;
    let mut zip = Zip::new(src.at);
    zip.add("README.txt", README.as_bytes());
    zip.add("diagnostics.txt", redact(&src.diagnostics, home).as_bytes());
    if let Some(raw) = &src.settings {
        let text = match serde_json::from_str::<Value>(raw) {
            Ok(v) => serde_json::to_string_pretty(&clean(v, home)).unwrap_or_default(),
            Err(_) => "(not JSON)".into(),
        };
        zip.add("settings.json", text.as_bytes());
    }
    for (name, text) in &src.logs {
        let lines: Vec<&str> = text.lines().collect();
        let tail = lines[lines.len().saturating_sub(LOG_LINES)..].join("\n");
        zip.add(&format!("logs/{}", safe_name(name)), redact(&tail, home).as_bytes());
    }
    let Some(dir) = src.session else { return zip.finish() };
    for name in ["progress.json", "events.jsonl", "steps.jsonl"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(name)) {
            zip.add(&format!("session/{name}"), clean_lines(&text, home).as_bytes());
        }
    }
    let mut pictures = 0;
    for (folder, ext) in [("shots", "jpg"), ("tray", "png")] {
        let Ok(read) = std::fs::read_dir(dir.join(folder)) else { continue };
        let mut files: Vec<_> = read
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == ext))
            .collect();
        files.sort();
        for path in files {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            let Ok(data) = std::fs::read(&path) else { continue };
            if pictures + data.len() > PICTURES {
                break;
            }
            pictures += data.len();
            zip.add(&format!("{folder}/{}", safe_name(name)), &data);
        }
    }
    zip.finish()
}

/// A file name made of letters, digits, `-`, `_` and `.` only.
fn safe_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' })
        .collect()
}

/// The app's logs: every `*.log` in its folder and the folders right
/// below (`claude/*.log`), each named by where it lies.
pub fn logs(dirs: &[&Path]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for root in dirs {
        let mut folders = vec![(root.to_path_buf(), String::new())];
        if let Ok(read) = std::fs::read_dir(root) {
            for e in read.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() && name != "session" {
                    folders.push((e.path(), format!("{name}-")));
                }
            }
        }
        for (folder, prefix) in folders {
            let Ok(read) = std::fs::read_dir(&folder) else { continue };
            for e in read.flatten() {
                let path = e.path();
                if !path.is_file() || path.extension().is_none_or(|x| x != "log") {
                    continue;
                }
                let name = format!("{prefix}{}", e.file_name().to_string_lossy());
                if let Ok(text) = std::fs::read_to_string(&path) {
                    found.push((name, text));
                }
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names in a zip's central directory, and each entry's data.
    fn entries(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
        let end = zip.len() - 22;
        assert_eq!(&zip[end..end + 4], &0x0605_4b50u32.to_le_bytes());
        let count = u16::from_le_bytes([zip[end + 10], zip[end + 11]]) as usize;
        let mut at = u32::from_le_bytes(zip[end + 16..end + 20].try_into().unwrap()) as usize;
        let mut found = Vec::new();
        for _ in 0..count {
            assert_eq!(&zip[at..at + 4], &0x0201_4b50u32.to_le_bytes());
            let u16_at = |i: usize| u16::from_le_bytes([zip[at + i], zip[at + i + 1]]) as usize;
            let u32_at = |i: usize| u32::from_le_bytes(zip[at + i..at + i + 4].try_into().unwrap());
            let (crc, size, name_len) = (u32_at(16), u32_at(20) as usize, u16_at(28));
            let local = u32_at(42) as usize;
            let name = String::from_utf8(zip[at + 46..at + 46 + name_len].to_vec()).unwrap();
            let data_at = local + 30 + name_len;
            let data = zip[data_at..data_at + size].to_vec();
            assert_eq!(crc32(&data), crc, "{name}");
            found.push((name, data));
            at += 46 + name_len;
        }
        found
    }

    fn at() -> time::PrimitiveDateTime {
        let day = time::Date::from_calendar_date(2026, time::Month::October, 28).unwrap();
        time::PrimitiveDateTime::new(day, time::Time::from_hms(9, 30, 0).unwrap())
    }

    #[test]
    fn a_png_has_its_chunks_and_its_pixels() {
        let rgba = [255u8, 0, 0, 255].repeat(4);
        let png = png(2, 2, &rgba);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // Two rows, each a filter byte and two pixels, stored as they are.
        let idat = png.windows(4).position(|w| w == b"IDAT").unwrap() + 4;
        assert_eq!(&png[idat..idat + 2], &[0x78, 0x01]);
        assert_eq!(&png[idat + 7..idat + 7 + 9], &[0, 255, 0, 0, 255, 255, 0, 0, 255]);
    }

    #[test]
    fn crc32_is_the_zip_one() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn a_zip_lists_what_was_added() {
        let mut zip = Zip::new(at());
        zip.add("a.txt", b"one");
        zip.add("dir/b.bin", &[0, 1, 2]);
        let found = entries(&zip.finish());
        assert_eq!(found, [("a.txt".into(), b"one".to_vec()), ("dir/b.bin".into(), vec![0, 1, 2])]);
    }

    #[test]
    fn redacts_emails_paths_and_tokens_but_keeps_the_rest() {
        let home = r"C:\Users\Lan";
        assert_eq!(
            redact(r"switch 2 ok for lan@example.com at C:\Users\Lan\.claude in 1.2s", home),
            r"switch 2 ok for [email] at ~\.claude in 1.2s"
        );
        assert_eq!(
            redact(r#"{"accessToken":"sk-ant-oat01-abcdefghijklmnop"} D:\work"#, home),
            "[secret] [path]"
        );
        assert_eq!(
            redact(
                "Bearer eyJhbGciOiJIUzI1NiJ9.e30.abc and 0123456789abcdef0123456789abcdef",
                home
            ),
            "Bearer [secret] and [secret]"
        );
        assert_eq!(
            redact("claude: kept 2, switched to 1", home),
            "claude: kept 2, switched to 1"
        );
        assert_eq!(
            redact("https://github.com/shin315/quota-station/releases", home),
            "https://github.com/shin315/quota-station/releases"
        );
    }

    #[test]
    fn json_hides_names_and_secrets_by_key() {
        let v = serde_json::json!({
            "machineName": "Lan-Desk",
            "label": [{"host": "mac.local", "user": "lan"}],
            "theme": "dark",
            "nested": {"refresh_token": "abc", "count": 3, "email": null},
        });
        let v = clean(v, "/Users/lan");
        assert_eq!(v["machineName"], "[hidden]");
        assert_eq!(v["label"], "[hidden]");
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["nested"]["refresh_token"], "[hidden]");
        assert_eq!(v["nested"]["count"], 3);
        assert_eq!(v["nested"]["email"], Value::Null);
    }

    /// Fake secrets in every place the app keeps something: none reaches
    /// the zip, whether the file is picked (settings, logs, the session)
    /// or merely lies next to the ones that are (logins, sealed secrets).
    #[test]
    fn a_bundle_from_fixtures_holds_none_of_their_secrets() {
        let root = std::env::temp_dir().join(format!("qs-diag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("Users").join("Lan");
        let app = home.join("AppData").join("Roaming").join("com.shin315.quota-station");
        let session = app.join("session");
        for dir in [
            app.join("claude"),
            app.join("secrets"),
            session.join("shots"),
            session.join("tray"),
            home.join(".claude"),
        ] {
            std::fs::create_dir_all(&dir).unwrap();
        }
        let secrets = [
            "lan.nguyen@example.com",
            "sk-ant-oat01-FAKEFAKEFAKEFAKE",
            "sk-ant-ort01-REFRESHFAKEFAKE",
            "eyJhbGciOiJSUzI1NiJ9.FAKEPAYLOAD.FAKESIG",
            "ya29.FAKEGOOGLEACCESS",
            "Lan-Gaming-PC",
            "sessionKey=FAKECOOKIE0123456789abcdefFAKE",
            r"C:\Users\Lan",
            "D:\\private\\notes",
        ];
        std::fs::write(
            app.join("settings.json"),
            serde_json::json!({
                "machineName": "Lan-Gaming-PC",
                "codexHome": "D:\\private\\notes",
                "backupDir": format!("{}\\OneDrive\\Quota Station Agents", home.display()),
                "label": [{"name": "lan.nguyen@example.com", "host": "Lan-Gaming-PC"}],
                "theme": "dark",
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            app.join("claude").join("switch.log"),
            format!(
                "2026-10-28T09:00:00Z switched to lan.nguyen@example.com\n\
                 2026-10-28T09:03:00Z refreshed sk-ant-oat01-FAKEFAKEFAKEFAKE (sk-ant-ort01-REFRESHFAKEFAKE)\n\
                 2026-10-28T09:06:00Z read failed at {}\\.claude\\x Authorization: Bearer eyJhbGciOiJSUzI1NiJ9.FAKEPAYLOAD.FAKESIG\n\
                 2026-10-28T09:09:00Z cookie sessionKey=FAKECOOKIE0123456789abcdefFAKE ya29.FAKEGOOGLEACCESS\n",
                home.display()
            ),
        )
        .unwrap();
        std::fs::write(
            session.join("events.jsonl"),
            "{\"kind\":\"note\",\"email\":\"lan.nguyen@example.com\",\"text\":\"token sk-ant-oat01-FAKEFAKEFAKEFAKE\"}\n\
             not json lan.nguyen@example.com\n",
        )
        .unwrap();
        std::fs::write(
            session.join("steps.jsonl"),
            "{\"id\":\"m1-start\",\"facts\":{\"machine\":\"Lan-Gaming-PC\",\"accounts\":2}}\n",
        )
        .unwrap();
        std::fs::write(session.join("shots").join("01-m1-tray-1.jpg"), [0xff, 0xd8, 0xff]).unwrap();
        // Files the bundle must never pick, though they lie beside the others.
        std::fs::write(
            home.join(".claude").join(".credentials.json"),
            "{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-FAKEFAKEFAKEFAKE\"}}",
        )
        .unwrap();
        std::fs::write(app.join("secrets").join("x.bin"), "ya29.FAKEGOOGLEACCESS").unwrap();
        std::fs::write(
            app.join("auth.json"),
            "{\"OPENAI_API_KEY\":\"sk-ant-ort01-REFRESHFAKEFAKE\"}",
        )
        .unwrap();

        let home_text = home.display().to_string();
        let zip = bundle(&Sources {
            home: &home_text,
            at: at(),
            diagnostics: "Quota Station 0.1.0 · Windows 11 24H2\nClaude quota: ok · read lan.nguyen@example.com".into(),
            settings: std::fs::read_to_string(app.join("settings.json")).ok(),
            logs: logs(&[app.as_path()]),
            session: Some(&session),
        });
        let _ = std::fs::remove_dir_all(&root);

        let all = String::from_utf8_lossy(&zip).into_owned();
        for secret in secrets {
            // As written, and as JSON writes it (`\\` for `\`).
            let escaped = serde_json::to_string(secret).unwrap();
            assert!(!all.contains(secret), "{secret} is in the zip");
            assert!(!all.contains(escaped.trim_matches('"')), "{secret} is in the zip");
        }
        assert!(!all.contains(&home_text), "the home folder is in the zip");
        let names: Vec<String> = entries(&zip).into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "README.txt",
                "diagnostics.txt",
                "settings.json",
                "logs/claude-switch.log",
                "session/events.jsonl",
                "session/steps.jsonl",
                "shots/01-m1-tray-1.jpg",
            ]
        );
        // What is not secret stays readable.
        assert!(all.contains("\"theme\": \"dark\""));
        assert!(all.contains("switched to [email]"));
        assert!(all.contains("\"accounts\":2"));
    }
}
