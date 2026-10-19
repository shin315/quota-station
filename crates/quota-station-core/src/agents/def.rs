//! One MCP server as the app carries it between agents: how it runs (a
//! command on this Mac, or an address) and its environment or headers, with
//! the agent's own spelling left to [`super::claude_mcp`],
//! [`super::codex_mcp`] and [`super::agy_mcp`].
//!
//! Some values are secrets: a token in the environment, an `Authorization`
//! header, a key after `--api-key`, an address that carries one. They are
//! found by name and by shape ([`Def::slots`]), each under a slot id
//! (`env:NAME`, `header:NAME`, `arg:2`, `url`) that also names its Keychain
//! item ([`super::secrets`]). A `Def` holding real values lives only in
//! memory while a write runs; what is shown, logged, kept in the app's
//! store or in a backup is [`Def::masked`] or [`Def::stripped`]. `Debug`
//! prints no value at all.

use serde::{Deserialize, Serialize};

/// One `NAME = value` of an environment or of headers, in the file's order.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    /// `name` in JSON, so the app's own files never put a secret's value
    /// under a key called `key`.
    #[serde(rename = "name")]
    pub key: String,
    pub value: String,
}

/// The name only: a value may be a secret.
impl std::fmt::Debug for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = …", self.key)
    }
}

impl Pair {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Pair { key: key.into(), value: value.into() }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Def {
    /// A command the agent starts on this Mac.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: Vec<Pair>,
        #[serde(default)]
        cwd: Option<String>,
    },
    /// An address the agent connects to; `sse` for the older event stream,
    /// which Codex does not speak.
    Http {
        url: String,
        #[serde(default)]
        headers: Vec<Pair>,
        #[serde(default)]
        sse: bool,
    },
}

impl std::fmt::Debug for Def {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let keys = |pairs: &[Pair]| pairs.iter().map(|p| p.key.clone()).collect::<Vec<_>>();
        match self {
            Def::Stdio { command, args, env, cwd } => f
                .debug_struct("Stdio")
                .field("command", command)
                .field("args", &args.len())
                .field("env", &keys(env))
                .field("cwd", &cwd.is_some())
                .finish(),
            Def::Http { headers, sse, .. } => {
                f.debug_struct("Http").field("headers", &keys(headers)).field("sse", sse).finish()
            }
        }
    }
}

/// What the preview and the backups show instead of a secret.
pub const MASK: &str = "••••••";
/// What a backup or the app's store holds instead of a secret.
pub const REDACTED: &str = "<redacted>";

/// Words in a name that say its value is a secret.
const SECRET_WORDS: &[&str] = &[
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "auth",
    "credential",
    "cookie",
    "bearer",
    "session",
    "signature",
];
/// Endings that make such a name hold a place rather than the secret.
const PLACE_ENDINGS: &[&str] = &["_path", "_paths", "_dir", "_file", "_home", "_url", "_name"];
/// How common tokens start, whatever the name they sit under.
const TOKEN_STARTS: &[&str] = &[
    "sk-",
    "sk_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "xapp-",
    "AKIA",
    "AIza",
    "ya29.",
];

/// A name whose value is a secret: `GITHUB_TOKEN`, `Authorization`,
/// `x-api-key`, `--api-key`; not `KEY_FILE` or `AUTH_URL`.
pub fn secret_name(name: &str) -> bool {
    let n = name.trim_start_matches('-').to_ascii_lowercase();
    let n = n.replace('-', "_");
    SECRET_WORDS.iter().any(|w| n.contains(w)) && !PLACE_ENDINGS.iter().any(|e| n.ends_with(e))
}

/// A value that has the shape of a token whatever its name.
pub fn token_shaped(value: &str) -> bool {
    let v = value.trim();
    let v = v.strip_prefix("Bearer ").unwrap_or(v);
    TOKEN_STARTS.iter().any(|s| v.starts_with(s) && v.len() >= s.len() + 8)
        || (v.starts_with("eyJ") && v.len() >= 24 && v.matches('.').count() == 2)
}

/// Only references to environment variables, which the agent fills in:
/// `${TOKEN}`, `Bearer ${TOKEN}`, `$TOKEN`. Not a secret itself.
pub fn only_references(value: &str) -> bool {
    let mut rest = String::new();
    let mut refs = 0;
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            rest.push(c);
            continue;
        }
        if chars.peek() == Some(&'{') {
            let mut closed = false;
            for c in chars.by_ref() {
                if c == '}' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return false;
            }
        } else {
            let mut any = false;
            while chars.peek().is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_') {
                chars.next();
                any = true;
            }
            if !any {
                return false;
            }
        }
        refs += 1;
    }
    let rest = rest.trim();
    refs > 0 && rest.len() <= 10 && rest.chars().all(|c| c.is_ascii_alphabetic() || c == ' ')
}

/// The value under `name` is a secret.
pub fn secret_value(name: &str, value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() || v == REDACTED || only_references(v) {
        return false;
    }
    secret_name(name) || token_shaped(v)
}

/// An address that carries a secret: a password before the host, or a
/// query parameter with a secret's name.
pub fn secret_url(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else { return token_shaped(url) };
    u.password().is_some_and(|p| !p.is_empty())
        || u.query_pairs().any(|(k, v)| secret_value(&k, &v))
        || u.path_segments().into_iter().flatten().any(token_shaped)
}

/// For each argument, whether it is a secret: the value after a flag with a
/// secret's name (`--api-key VALUE`, `--token=VALUE`), or one shaped like a
/// token.
pub fn secret_args(args: &[String]) -> Vec<bool> {
    let mut out = Vec::with_capacity(args.len());
    for (i, a) in args.iter().enumerate() {
        let after_flag = i > 0 && {
            let prev = &args[i - 1];
            prev.starts_with('-') && !prev.contains('=') && secret_name(prev)
        };
        let joined =
            a.starts_with('-') && a.split_once('=').is_some_and(|(k, v)| secret_value(k, v));
        let value = !a.starts_with('-') && (after_flag && !a.trim().is_empty());
        out.push(value && !only_references(a) || joined || token_shaped(a));
    }
    out
}

/// Where a secret sits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    Env,
    Header,
    Arg,
    Url,
}

/// One secret of a definition: the slot id and the name to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub id: String,
    /// The variable or header name, the flag before the argument, or `URL`.
    pub label: String,
    pub place: Place,
}

impl Def {
    pub fn is_http(&self) -> bool {
        matches!(self, Def::Http { .. })
    }

    pub fn is_sse(&self) -> bool {
        matches!(self, Def::Http { sse: true, .. })
    }

    /// The secrets in it, in a stable order.
    pub fn slots(&self) -> Vec<Slot> {
        let mut out = Vec::new();
        match self {
            Def::Stdio { args, env, .. } => {
                for (i, secret) in secret_args(args).into_iter().enumerate() {
                    if secret {
                        let label = match args[i].split_once('=') {
                            Some((flag, _)) if args[i].starts_with('-') => flag.to_string(),
                            _ if i > 0 && args[i - 1].starts_with('-') => args[i - 1].clone(),
                            _ => format!("tham số {}", i + 1),
                        };
                        out.push(Slot { id: format!("arg:{i}"), label, place: Place::Arg });
                    }
                }
                for p in env.iter().filter(|p| secret_value(&p.key, &p.value)) {
                    out.push(Slot {
                        id: format!("env:{}", p.key),
                        label: p.key.clone(),
                        place: Place::Env,
                    });
                }
            }
            Def::Http { url, headers, .. } => {
                if secret_url(url) {
                    out.push(Slot { id: "url".into(), label: "URL".into(), place: Place::Url });
                }
                for p in headers.iter().filter(|p| secret_value(&p.key, &p.value)) {
                    out.push(Slot {
                        id: format!("header:{}", p.key),
                        label: p.key.clone(),
                        place: Place::Header,
                    });
                }
            }
        }
        out
    }

    /// The value at a slot.
    pub fn get(&self, slot: &str) -> Option<&str> {
        match (self, slot.split_once(':')) {
            (Def::Stdio { env, .. }, Some(("env", k))) => {
                env.iter().find(|p| p.key == k).map(|p| p.value.as_str())
            }
            (Def::Stdio { args, .. }, Some(("arg", i))) => {
                let a = args.get(i.parse::<usize>().ok()?)?;
                Some(match a.split_once('=') {
                    Some((_, v)) if a.starts_with('-') => v,
                    _ => a.as_str(),
                })
            }
            (Def::Http { headers, .. }, Some(("header", k))) => {
                headers.iter().find(|p| p.key == k).map(|p| p.value.as_str())
            }
            (Def::Http { url, .. }, None) if slot == "url" => Some(url),
            _ => None,
        }
    }

    /// Puts `value` at a slot; `false` when the definition has no such place.
    pub fn set(&mut self, slot: &str, value: &str) -> bool {
        match (self, slot.split_once(':')) {
            (Def::Stdio { env, .. }, Some(("env", k))) => match env.iter_mut().find(|p| p.key == k)
            {
                Some(p) => {
                    p.value = value.to_string();
                    true
                }
                None => false,
            },
            (Def::Stdio { args, .. }, Some(("arg", i))) => {
                let Some(a) = i.parse::<usize>().ok().and_then(|i| args.get_mut(i)) else {
                    return false;
                };
                *a = match a.split_once('=') {
                    Some((flag, _)) if a.starts_with('-') => format!("{flag}={value}"),
                    _ => value.to_string(),
                };
                true
            }
            (Def::Http { headers, .. }, Some(("header", k))) => {
                match headers.iter_mut().find(|p| p.key == k) {
                    Some(p) => {
                        p.value = value.to_string();
                        true
                    }
                    None => false,
                }
            }
            (Def::Http { url, .. }, None) if slot == "url" => {
                *url = value.to_string();
                true
            }
            _ => false,
        }
    }

    /// Every secret replaced by `with`, for the preview ([`MASK`]) and for
    /// backups ([`REDACTED`]). An address keeps its host so it still says
    /// where it goes.
    pub fn masked(&self, with: &str) -> Def {
        let mut out = self.clone();
        for slot in self.slots() {
            let value =
                if slot.place == Place::Url { mask_url(self.url(), with) } else { with.into() };
            out.set(&slot.id, &value);
        }
        out
    }

    /// Every secret emptied: what the app's store keeps. The slots come
    /// back from [`Def::slots`] of the original, not of this.
    pub fn stripped(&self) -> Def {
        self.masked(REDACTED)
    }

    fn url(&self) -> &str {
        match self {
            Def::Http { url, .. } => url,
            Def::Stdio { .. } => "",
        }
    }

    /// The command, or the package `npx` and the like run: what the matrix
    /// shows under the name, and what "lệnh chưa cài" is about.
    pub fn program(&self) -> Option<String> {
        let Def::Stdio { command, args, .. } = self else { return None };
        let base = command.rsplit('/').next().unwrap_or(command);
        let runner = ["npx", "uvx", "bunx", "pnpx", "node", "deno", "python3", "python", "uv"];
        if runner.contains(&base) {
            let pkg = args.iter().find(|a| !a.starts_with('-') && *a != "run" && *a != "dlx");
            if let Some(p) = pkg {
                let name = p.rsplit('/').next().unwrap_or(p);
                // `pkg@1.4.0` and `@scope/pkg@latest` name the package.
                let name = match name.rfind('@') {
                    Some(at) if at > 0 => &name[..at],
                    _ => name,
                };
                return Some(name.to_string());
            }
        }
        Some(base.to_string())
    }

    /// The host an address goes to.
    pub fn host(&self) -> Option<String> {
        let Def::Http { url, .. } = self else { return None };
        url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_string))
    }

    /// The same server, whatever the order of its environment or headers.
    pub fn same(&self, other: &Def) -> bool {
        fn sorted(pairs: &[Pair]) -> Vec<(&str, &str)> {
            let mut v: Vec<(&str, &str)> =
                pairs.iter().map(|p| (p.key.as_str(), p.value.as_str())).collect();
            v.sort();
            v
        }
        match (self, other) {
            (
                Def::Stdio { command: c1, args: a1, env: e1, cwd: d1 },
                Def::Stdio { command: c2, args: a2, env: e2, cwd: d2 },
            ) => c1 == c2 && a1 == a2 && d1 == d2 && sorted(e1) == sorted(e2),
            (
                Def::Http { url: u1, headers: h1, sse: s1 },
                Def::Http { url: u2, headers: h2, sse: s2 },
            ) => u1 == u2 && s1 == s2 && sorted(h1) == sorted(h2),
            _ => false,
        }
    }
}

/// The address with its password, secret query values and token-shaped
/// path parts replaced. One the parser refuses is replaced whole.
fn mask_url(url: &str, with: &str) -> String {
    // Plain letters survive the URL's escaping; the mask goes in after.
    const STANDIN: &str = "QSMASKQS";
    let Ok(mut u) = url::Url::parse(url) else { return with.to_string() };
    if u.password().is_some() {
        let _ = u.set_password(Some(STANDIN));
    }
    if u.query_pairs().any(|(k, v)| secret_value(&k, &v)) {
        let pairs: Vec<(String, String)> = u
            .query_pairs()
            .map(|(k, v)| {
                let v = if secret_value(&k, &v) { STANDIN.to_string() } else { v.to_string() };
                (k.to_string(), v)
            })
            .collect();
        u.query_pairs_mut().clear().extend_pairs(pairs);
    }
    if u.path_segments().into_iter().flatten().any(token_shaped) {
        let path: Vec<String> = u
            .path_segments()
            .into_iter()
            .flatten()
            .map(|s| if token_shaped(s) { STANDIN.to_string() } else { s.to_string() })
            .collect();
        u.set_path(&path.join("/"));
    }
    // A mask already there comes back escaped; it reads as it was.
    let escaped: String =
        with.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
    u.to_string().replace(STANDIN, with).replace(&escaped, with)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stdio(args: &[&str], env: &[(&str, &str)]) -> Def {
        Def::Stdio {
            command: "npx".into(),
            args: args.iter().map(|a| a.to_string()).collect(),
            env: env.iter().map(|(k, v)| Pair::new(*k, *v)).collect(),
            cwd: None,
        }
    }

    #[test]
    fn secrets_are_found_by_name_and_by_shape() {
        assert!(secret_name("GITHUB_TOKEN"));
        assert!(secret_name("Authorization"));
        assert!(secret_name("x-api-key"));
        assert!(secret_name("--api-key"));
        assert!(!secret_name("KEY_FILE"));
        assert!(!secret_name("AUTH_URL"));
        assert!(!secret_name("NODE_ENV"));
        assert!(secret_value("NODE_ENV", "sk-test-not-a-real-key-123"));
        assert!(!secret_value("API_TOKEN", "${API_TOKEN}"));
        assert!(!secret_value("Authorization", "Bearer ${API_TOKEN}"));
        assert!(secret_value("Authorization", "Bearer abcdefabcdef"));
        assert!(!secret_value("API_TOKEN", ""));
        assert!(secret_url("https://u:pw@example.com/mcp"));
        assert!(secret_url("https://example.com/mcp?api_key=abc123"));
        assert!(!secret_url("https://example.com/mcp?mode=fast"));
    }

    #[test]
    fn slots_cover_env_args_headers_and_addresses() {
        let d = stdio(
            &["-y", "files-mcp", "--api-key", "k-123", "--token=t-456", "--root", "/w"],
            &[("FILES_TOKEN", "v"), ("MODE", "x")],
        );
        let ids: Vec<String> = d.slots().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["arg:3", "arg:4", "env:FILES_TOKEN"]);
        assert_eq!(d.get("arg:4"), Some("t-456"));
        let masked = d.masked(MASK);
        let Def::Stdio { args, env, .. } = &masked else { unreachable!() };
        assert_eq!(args[3], MASK);
        assert_eq!(args[4], format!("--token={MASK}"));
        assert_eq!(args[6], "/w");
        assert_eq!(env[0].value, MASK);
        assert_eq!(env[1].value, "x");

        let h = Def::Http {
            url: "https://example.com/mcp?key=s3cr3t&mode=fast".into(),
            headers: vec![Pair::new("Authorization", "Bearer abcdefabcdef")],
            sse: false,
        };
        let m = h.masked(MASK);
        let Def::Http { url, headers, .. } = &m else { unreachable!() };
        assert!(
            !url.contains("s3cr3t") && url.contains("example.com") && url.contains("mode=fast"),
            "{url}"
        );
        assert_eq!(headers[0].value, MASK);
    }

    #[test]
    fn debug_prints_no_value() {
        let d = stdio(&["--api-key", "k-123"], &[("FILES_TOKEN", "t-456")]);
        let text = format!("{d:?}");
        assert!(!text.contains("k-123") && !text.contains("t-456"), "{text}");
    }

    #[test]
    fn the_program_is_the_package_npx_runs() {
        assert_eq!(
            stdio(&["-y", "@acme/browser-mcp@1.4.0"], &[]).program().unwrap(),
            "browser-mcp"
        );
        let d = Def::Stdio {
            command: "/opt/x/bin/files-mcp".into(),
            args: vec![],
            env: vec![],
            cwd: None,
        };
        assert_eq!(d.program().unwrap(), "files-mcp");
    }
}
