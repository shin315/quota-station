//! Fakes for the Claude account tests: a temporary home with a made-up
//! Claude Code login, a `security` that keeps generic passwords as files
//! (and reads `security -i` lines from stdin, as the app writes them), a
//! `launchctl` that keeps which labels are loaded, and a local Anthropic
//! that answers the usage and token calls. Nothing here reaches the
//! Keychain, launchd or Anthropic, and every login is made up.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use quota_station_core::claude::keychain::Security;
use quota_station_core::claude::store::{self, Account, Kind, Store};
use quota_station_core::claude::usage::Endpoints;
use quota_station_core::claude::Paths;
use serde_json::{json, Value};

use super::{script, temp_dir};

/// A `security` that keeps items as files in `dir/items/<service>--<account>`,
/// exits 44 for a missing one as macOS's does, takes a write from a
/// `security -i` line on stdin (the value as hex after `-X`), and records
/// every run (never a value) in `dir/calls`. With `dir/watch` naming a
/// folder, each write notes in `dir/watched` whether that folder existed;
/// with `dir/drop-add` naming a service, writes to it report success and
/// keep nothing.
pub fn fake_security(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("items")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/security"),
        &format!(
            r#"if [ "$1" = "-i" ]; then
  IFS= read -r line
  eval "set -- $line"
fi
cmd="$1"; shift
printf '%s' "$cmd" >> '{d}/calls'
svc=""; acct=""; hex=""; value=""; show=""
while [ $# -gt 0 ]; do
  case "$1" in
    -s) svc="$2"; printf ' -s %s' "$2" >> '{d}/calls'; shift 2 ;;
    -a) acct="$2"; printf ' -a %s' "$2" >> '{d}/calls'; shift 2 ;;
    -X) hex="$2"; shift 2 ;;
    -w) if [ $# -ge 2 ] && [ "$cmd" = add-generic-password ]; then value="$2"; shift 2; else show=1; shift; fi ;;
    *) shift ;;
  esac
done
printf '\n' >> '{d}/calls'
item='{d}/items/'"$svc--$acct"
missing() {{ echo "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain." >&2; exit 44; }}
case "$cmd" in
  add-generic-password)
    [ -e '{d}/fail-add' ] && {{ echo "security: SecKeychainItemCreateFromContent: User interaction is not allowed." >&2; exit 36; }}
    [ -e '{d}/watch' ] && {{ [ -d "$(cat '{d}/watch')" ] && echo "held $svc" || echo "free $svc"; }} >> '{d}/watched'
    [ -e '{d}/drop-add' ] && [ "$svc" = "$(cat '{d}/drop-add')" ] && exit 0
    if [ -n "$hex" ]; then printf '%s' "$hex" | /usr/bin/xxd -r -p > "$item"; else printf '%s' "$value" > "$item"; fi ;;
  find-generic-password)
    [ -e '{d}/fail-find' ] && {{ echo "security: SecKeychainSearchCopyNext: User interaction is not allowed." >&2; exit 36; }}
    [ -f "$item" ] || missing
    if [ -n "$show" ]; then cat "$item"; echo; fi ;;
  delete-generic-password)
    [ -f "$item" ] || missing
    rm "$item" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}

/// A `launchctl` that keeps loaded labels as files in `dir/loaded/`:
/// `list LABEL` exits 0 or 113, `bootstrap DOMAIN PLIST` loads the plist's
/// label, `bootout DOMAIN/LABEL` unloads it. Every run is recorded.
pub fn fake_launchctl(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("loaded")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/launchctl"),
        &format!(
            r#"printf '%s\n' "$*" >> '{d}/launchctl-calls'
case "$1" in
  list) [ -e '{d}/loaded/'"$2" ] && exit 0; exit 113 ;;
  bootstrap) label=$(basename "$3" .plist); : > '{d}/loaded/'"$label" ;;
  bootout) label=${{2##*/}}; [ -e '{d}/loaded/'"$label" ] || exit 3; rm '{d}/loaded/'"$label" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}

/// A request the fake Anthropic answered.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    pub fn header(&self, key: &str) -> Option<String> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.clone())
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

/// How the fake answers.
#[derive(Clone, Debug, Default)]
pub struct Answers {
    /// `/api/oauth/usage` by access token: status, body, extra headers.
    pub usage: BTreeMap<String, (u16, String, Vec<(String, String)>)>,
    /// `/v1/oauth/token` by refresh token: status and body.
    pub token: BTreeMap<String, (u16, String)>,
}

pub struct FakeAnthropic {
    pub base: String,
    pub answers: Arc<Mutex<Answers>>,
    pub requests: Arc<Mutex<Vec<Request>>>,
}

impl FakeAnthropic {
    pub fn start() -> FakeAnthropic {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let answers = Arc::new(Mutex::new(Answers::default()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (a, r) = (answers.clone(), requests.clone());
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (a, r) = (a.clone(), r.clone());
                thread::spawn(move || serve(stream, &a, &r));
            }
        });
        FakeAnthropic { base, answers, requests }
    }

    pub fn endpoints(&self) -> Endpoints {
        Endpoints::fake(&self.base)
    }

    /// The usage answer for `access`: the share used of each window.
    pub fn usage(&self, access: &str, five: f64, five_reset: Option<&str>, week: Option<f64>, fable: Option<f64>) {
        let mut body = json!({ "five_hour": { "utilization": five, "resets_at": five_reset } });
        if let Some(w) = week {
            body["seven_day"] = json!({ "utilization": w, "resets_at": "2030-01-19T14:00:00Z" });
        }
        if let Some(f) = fable {
            body["limits"] = json!([{ "scope": { "model": { "display_name": "Fable" } }, "percent": f, "resets_at": "2030-01-20T19:00:00Z" }]);
        }
        self.answers.lock().unwrap().usage.insert(access.into(), (200, body.to_string(), Vec::new()));
    }

    pub fn usage_status(&self, access: &str, status: u16, headers: &[(&str, &str)]) {
        let headers = headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        self.answers.lock().unwrap().usage.insert(access.into(), (status, "{}".into(), headers));
    }

    /// A refresh of `refresh` gives `access` and `next` for one hour.
    pub fn refreshes(&self, refresh: &str, access: &str, next: &str) {
        let body = json!({ "access_token": access, "refresh_token": next, "expires_in": 3600 }).to_string();
        self.answers.lock().unwrap().token.insert(refresh.into(), (200, body));
    }

    pub fn refuses(&self, refresh: &str, error: &str) {
        let body = json!({ "error": error }).to_string();
        self.answers.lock().unwrap().token.insert(refresh.into(), (400, body));
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    pub fn to(&self, path: &str) -> Vec<Request> {
        self.requests().into_iter().filter(|r| r.path == path).collect()
    }
}

fn serve(stream: TcpStream, answers: &Mutex<Answers>, requests: &Mutex<Vec<Request>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.trim_end().split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    let _ = reader.read_exact(&mut body);
    let request = Request { method, path, headers, body: String::from_utf8_lossy(&body).into_owned() };
    requests.lock().unwrap().push(request.clone());
    let answers = answers.lock().unwrap().clone();
    let (status, body, extra) = match request.path.as_str() {
        "/api/oauth/usage" => {
            let token = request.header("authorization").unwrap_or_default().trim_start_matches("Bearer ").to_string();
            answers.usage.get(&token).cloned().unwrap_or((401, r#"{"error":"unauthorized"}"#.into(), Vec::new()))
        }
        "/v1/oauth/token" => {
            let refresh = request.json()["refresh_token"].as_str().unwrap_or("").to_string();
            let (s, b) = answers.token.get(&refresh).cloned().unwrap_or((400, r#"{"error":"invalid_grant"}"#.into()));
            (s, b, Vec::new())
        }
        _ => (404, "{}".into(), Vec::new()),
    };
    let extra: String = extra.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    let reply = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream = stream;
    let _ = stream.write_all(reply.as_bytes());
}

/// A made-up credential, as Claude Code stores it.
pub fn credential(access: &str, refresh: &str, expires_at_ms: i64) -> String {
    json!({
        "claudeAiOauth": {
            "accessToken": access,
            "refreshToken": refresh,
            "expiresAt": expires_at_ms,
            "scopes": ["user:inference", "user:profile"],
            "subscriptionType": "team"
        }
    })
    .to_string()
}

/// Milliseconds since the epoch, `hours` from now.
pub fn in_hours(hours: f64) -> i64 {
    let now = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
    (now as f64 + hours * 3_600_000.0) as i64
}

pub fn oauth(email: &str, uuid: &str, org: &str) -> Value {
    json!({
        "emailAddress": email,
        "accountUuid": uuid,
        "organizationUuid": org,
        "organizationName": "Acme Studio",
        "displayName": "Lan"
    })
}

/// A Mac in a temporary folder.
pub struct Mac {
    pub home: PathBuf,
    pub dir: PathBuf,
    pub paths: Paths,
    pub store: Store,
    pub launchctl: PathBuf,
}

impl Mac {
    pub fn new(tag: &str) -> Mac {
        let home = temp_dir(tag);
        let dir = home.join("fakes");
        let security = Security::fake(fake_security(&dir));
        let launchctl = fake_launchctl(&dir);
        let mut paths = Paths::for_home(&home);
        // A lock that is held stays held: the tests never wait for one.
        paths.lock_wait = std::time::Duration::ZERO;
        let store = Store::new(paths.clone(), security);
        Mac { home, dir, paths, store, launchctl }
    }

    /// The value of a fake Keychain item.
    pub fn item(&self, service: &str, account: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join("items").join(format!("{service}--{account}"))).ok()
    }

    pub fn put_item(&self, service: &str, account: &str, value: &str) {
        std::fs::write(self.dir.join("items").join(format!("{service}--{account}")), value).unwrap();
    }

    /// Every `security` run, without values.
    pub fn security_calls(&self) -> String {
        std::fs::read_to_string(self.dir.join("calls")).unwrap_or_default()
    }

    pub fn user(&self) -> String {
        quota_station_core::claude::keychain::user_account()
    }

    /// Claude Code's live login: its Keychain item and `oauthAccount`.
    pub fn log_in(&self, credential: &str, oauth: &Value) {
        self.put_item("Claude Code-credentials", &self.user(), credential);
        let config = json!({ "numStartups": 12, "oauthAccount": oauth, "projects": { "/tmp/x": { "allowedTools": [] } } });
        std::fs::write(self.paths.claude_json(), serde_json::to_string_pretty(&config).unwrap()).unwrap();
    }

    pub fn live_credential(&self) -> Option<String> {
        self.item("Claude Code-credentials", &self.user())
    }

    pub fn live_email(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.paths.claude_json()).ok()?;
        let v: Value = serde_json::from_str(&text).ok()?;
        v["oauthAccount"]["emailAddress"].as_str().map(String::from)
    }

    /// Adds an account straight into the app's store.
    pub fn add(&self, slot: u32, email: &str, org: &str, credential: &str) {
        let oauth = oauth(email, &format!("uuid-{slot}"), org);
        let mut list = self.store.load().unwrap();
        list.put(Account {
            slot,
            alias: None,
            disabled: false,
            kind: Kind::Login,
            account_uuid: format!("uuid-{slot}"),
            organization_uuid: org.into(),
            identity: store::identity(email, org),
            added: "2026-10-24T00:00:00Z".into(),
        });
        self.store.save(&list).unwrap();
        self.store.set_credential(slot, credential).unwrap();
        self.store.set_oauth_account(slot, &oauth).unwrap();
    }

    pub fn load_label(&self, label: &str) {
        std::fs::write(self.dir.join("loaded").join(label), "").unwrap();
    }

    pub fn loaded(&self, label: &str) -> bool {
        self.dir.join("loaded").join(label).exists()
    }
}
