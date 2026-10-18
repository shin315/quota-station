//! Fakes for the Antigravity tests: a local Google (the consent page,
//! `/token`, `/userinfo`, and Cloud Code's `loadCodeAssist`,
//! `fetchAvailableModels` and `retrieveUserQuotaSummary` under `/daily` and
//! `/prod`), a browser that records the URL it is asked to open and follows
//! Google's redirect to the app's callback, and a `security` that records
//! its arguments and keeps items in a folder. Nothing here reaches Google or
//! the Keychain.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use quota_station_core::antigravity::cloud_code::Hosts;
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::antigravity::login::{Browser, Deps};
use quota_station_core::antigravity::oauth::Endpoints;
use quota_station_core::antigravity::store::Keychain;

use super::{script, temp_dir};

pub const CLIENT_ID: &str = "000000000000-quotastationtest.apps.googleusercontent.com";
pub const CLIENT_SECRET: &str = "test-secret-not-real";
pub const EMAIL: &str = "someone@example.com";
/// `antigravity_` and the MD5 of [`EMAIL`], worked out apart from the app.
pub const ACCOUNT_ID: &str = "antigravity_16d113840f999444259f73bac9ab8b10";
pub const ACCESS: &str = "fake-access-1";
pub const REFRESH: &str = "fake-refresh-1";

/// What the fake consent page does with the browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consent {
    /// Back to the callback with a code and the login's state.
    Code,
    /// Back with a code and someone else's state.
    WrongState,
    /// Back with no code, as when the owner cancels on Google's page.
    Denied,
}

#[derive(Clone, Debug)]
pub struct Script {
    pub consent: Consent,
    /// `/token` with `grant_type=authorization_code`: status and body.
    pub exchange: (u16, String),
    /// `/token` with `grant_type=refresh_token`.
    pub refresh: (u16, String),
    pub userinfo: (u16, String),
    /// Cloud Code: status and body. Status 0 closes the connection without
    /// an answer, as a network failure does.
    pub load_code_assist: (u16, String),
    pub models: (u16, String),
    pub summary: (u16, String),
    /// Answers given before the ones above, each once, in order for each
    /// call: the end of the path (`v1internal:fetchAvailableModels`), the
    /// status, the body.
    pub queue: Vec<(String, u16, String)>,
    /// How long every Cloud Code answer takes.
    pub delay: Duration,
}

/// A Cloud Code fixture: `tests/fixtures/cloud-code/<name>.json`.
pub fn cloud_code(name: &str) -> String {
    super::fixture_text(&format!("cloud-code/{name}.json"))
}

pub const LOAD_CODE_ASSIST: &str = "v1internal:loadCodeAssist";
pub const MODELS: &str = "v1internal:fetchAvailableModels";
pub const SUMMARY: &str = "v1internal:retrieveUserQuotaSummary";

pub fn token_body(access: &str, refresh: Option<&str>) -> String {
    let refresh = refresh.map(|r| format!(r#","refresh_token":"{r}""#)).unwrap_or_default();
    format!(
        r#"{{"access_token":"{access}","expires_in":3599,"token_type":"Bearer","id_token":"fake-id-token"{refresh},"scope":"openid"}}"#
    )
}

pub const INVALID_GRANT: &str =
    r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#;

impl Default for Script {
    fn default() -> Self {
        Script {
            consent: Consent::Code,
            exchange: (200, token_body(ACCESS, Some(REFRESH))),
            refresh: (200, token_body("fake-access-2", None)),
            userinfo: (
                200,
                format!(r#"{{"id":"1000001","email":"{EMAIL}","name":"Người Thử","picture":"x"}}"#),
            ),
            load_code_assist: (200, cloud_code("load-code-assist")),
            models: (200, cloud_code("fetch-available-models")),
            summary: (200, cloud_code("retrieve-user-quota-summary")),
            queue: Vec::new(),
            delay: Duration::ZERO,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    /// The JSON body.
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap()
    }

    /// `daily` or `prod` for a Cloud Code call.
    pub fn host(&self) -> &str {
        self.path.trim_start_matches('/').split('/').next().unwrap_or("")
    }

    pub fn param(&self, key: &str) -> Option<String> {
        self.query.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    pub fn form(&self) -> Vec<(String, String)> {
        url::form_urlencoded::parse(self.body.as_bytes()).into_owned().collect()
    }

    pub fn header(&self, key: &str) -> Option<String> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.clone())
    }
}

pub struct FakeGoogle {
    pub base: String,
    pub script: Arc<Mutex<Script>>,
    pub requests: Arc<Mutex<Vec<Request>>>,
    /// The most Cloud Code requests that were answered at once.
    pub most_at_once: Arc<AtomicUsize>,
}

impl FakeGoogle {
    pub fn start(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let script = Arc::new(Mutex::new(script));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let most_at_once = Arc::new(AtomicUsize::new(0));
        let now = Arc::new(AtomicUsize::new(0));
        let (s, r, m) = (script.clone(), requests.clone(), most_at_once.clone());
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (s, r, m, n) = (s.clone(), r.clone(), m.clone(), now.clone());
                thread::spawn(move || serve(stream, &s, &r, &m, &n));
            }
        });
        FakeGoogle { base, script, requests, most_at_once }
    }

    pub fn endpoints(&self) -> Endpoints {
        Endpoints::fake(&self.base)
    }

    pub fn hosts(&self) -> Hosts {
        Hosts::fake(&self.base)
    }

    /// The Cloud Code requests so far, in order.
    pub fn cloud_code(&self) -> Vec<Request> {
        self.requests().into_iter().filter(|r| r.path.contains("v1internal:")).collect()
    }

    /// The Cloud Code requests whose path ends with `call`.
    pub fn calls(&self, call: &str) -> Vec<Request> {
        self.requests().into_iter().filter(|r| r.path.ends_with(call)).collect()
    }

    pub fn clear(&self) {
        self.requests.lock().unwrap().clear();
    }

    pub fn set(&self, change: impl FnOnce(&mut Script)) {
        change(&mut self.script.lock().unwrap());
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    pub fn to(&self, path: &str) -> Vec<Request> {
        self.requests().into_iter().filter(|r| r.path == path).collect()
    }
}

fn serve(
    stream: TcpStream,
    script: &Mutex<Script>,
    requests: &Mutex<Vec<Request>>,
    most_at_once: &AtomicUsize,
    at_once: &AtomicUsize,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
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
    let url = url::Url::parse(&format!("http://fake{target}")).unwrap();
    let request = Request {
        method,
        path: url.path().to_string(),
        query: url.query_pairs().into_owned().collect(),
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    };
    requests.lock().unwrap().push(request.clone());
    if request.path.contains("v1internal:") {
        let n = at_once.fetch_add(1, Ordering::SeqCst) + 1;
        most_at_once.fetch_max(n, Ordering::SeqCst);
        let (delay, reply) = {
            let mut script = script.lock().unwrap();
            let queued = script.queue.iter().position(|(end, ..)| request.path.ends_with(end));
            let reply = match queued {
                Some(i) => {
                    let (_, status, body) = script.queue.remove(i);
                    (status, body)
                }
                None if request.path.ends_with(LOAD_CODE_ASSIST) => script.load_code_assist.clone(),
                None if request.path.ends_with(MODELS) => script.models.clone(),
                None if request.path.ends_with(SUMMARY) => script.summary.clone(),
                None => (404, r#"{"error":{"code":404,"message":"not found"}}"#.into()),
            };
            (script.delay, reply)
        };
        thread::sleep(delay);
        at_once.fetch_sub(1, Ordering::SeqCst);
        let (status, body) = reply;
        // Status 0: the connection closes with no answer.
        if status != 0 {
            let mut stream = stream;
            let _ = stream.write_all(json(status, &body).as_bytes());
        }
        return;
    }
    let script = script.lock().unwrap().clone();

    let reply = match request.path.as_str() {
        "/o/oauth2/v2/auth" => {
            let redirect = request.param("redirect_uri").unwrap_or_default();
            let state = request.param("state").unwrap_or_default();
            let location = match script.consent {
                Consent::Code => format!("{redirect}?state={state}&code=fake-code&scope=openid"),
                Consent::WrongState => format!("{redirect}?state=not-{state}&code=fake-code"),
                Consent::Denied => format!("{redirect}?error=access_denied&state={state}"),
            };
            format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        }
        "/token" => {
            let grant = request.form().into_iter().find(|(k, _)| k == "grant_type").map(|(_, v)| v);
            let (status, body) = if grant.as_deref() == Some("refresh_token") {
                script.refresh
            } else {
                script.exchange
            };
            json(status, &body)
        }
        "/oauth2/v2/userinfo" => json(script.userinfo.0, &script.userinfo.1),
        _ => json(404, r#"{"error":"not_found"}"#),
    };
    let mut stream = stream;
    let _ = stream.write_all(reply.as_bytes());
}

fn json(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// The page the app's callback showed the browser.
#[derive(Clone, Debug)]
pub struct Page {
    pub status: u16,
    pub body: String,
}

/// Records the URL; with `follow`, loads it as a browser would and follows
/// the consent page's redirect to the app's callback.
#[derive(Default)]
pub struct FakeBrowser {
    pub opened: Mutex<Vec<String>>,
    pub follow: bool,
    pub fail: bool,
    pub pages: Arc<Mutex<Vec<Page>>>,
}

impl FakeBrowser {
    pub fn following() -> Self {
        FakeBrowser { follow: true, ..Default::default() }
    }

    pub fn opened(&self) -> Vec<String> {
        self.opened.lock().unwrap().clone()
    }

    /// The callback pages loaded so far, after waiting up to 5 seconds for
    /// the first: the login can end before the browser has read its page.
    pub fn pages(&self) -> Vec<Page> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while self.pages.lock().unwrap().is_empty() && std::time::Instant::now() < deadline {
            thread::sleep(std::time::Duration::from_millis(10));
        }
        self.pages.lock().unwrap().clone()
    }
}

/// Where the consent page sends the browser, without going there.
pub fn redirect_of(url: &str) -> String {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client.get(url).send().unwrap();
    response.headers()["location"].to_str().unwrap().to_string()
}

impl Browser for FakeBrowser {
    fn open(&self, url: &str) -> Result<(), String> {
        self.opened.lock().unwrap().push(url.to_string());
        if self.fail {
            return Err("no browser".into());
        }
        if self.follow {
            let url = url.to_string();
            let pages = self.pages.clone();
            thread::spawn(move || {
                let callback = redirect_of(&url);
                let response = reqwest::blocking::get(&callback).unwrap();
                let status = response.status().as_u16();
                pages
                    .lock()
                    .unwrap()
                    .push(Page { status, body: response.text().unwrap_or_default() });
            });
        }
        Ok(())
    }
}

/// A temporary home with the app's folder, an `oauth-client.json` holding a
/// made-up client, and a fake `security`.
pub struct Home {
    pub home: PathBuf,
    pub app: AppDir,
    pub keychain: Keychain,
    /// The fake `security`'s folder: `calls` (one line per run, arguments
    /// split by U+001F) and `items/<service>--<account>`.
    pub sec: PathBuf,
}

impl Home {
    pub fn new(tag: &str) -> Self {
        let home = temp_dir(tag);
        let app = home.join("Library/Application Support/com.shin315.quota-station");
        std::fs::create_dir_all(&app).unwrap();
        let sec = home.join("fake-security");
        let program = fake_security(&sec);
        let home = Home { home, app: AppDir::new(app), keychain: Keychain::fake(program), sec };
        home.write_client(
            &format!(r#"{{"client_id":"{CLIENT_ID}","client_secret":"{CLIENT_SECRET}"}}"#),
            0o600,
        );
        home
    }

    pub fn write_client(&self, text: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        let file = self.app.path().join("oauth-client.json");
        std::fs::write(&file, text).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    pub fn deps<'a>(&'a self, endpoints: &'a Endpoints) -> Deps<'a> {
        Deps { app: &self.app, endpoints, keychain: &self.keychain }
    }

    pub fn index_file(&self) -> PathBuf {
        self.app.path().join("antigravity/accounts.json")
    }

    pub fn index_json(&self) -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(self.index_file()).unwrap()).unwrap()
    }

    /// Every `security` run so far, as argument lists.
    pub fn security_calls(&self) -> Vec<Vec<String>> {
        let text = std::fs::read_to_string(self.sec.join("calls")).unwrap_or_default();
        text.lines()
            .map(|l| l.split('\u{1f}').filter(|a| !a.is_empty()).map(String::from).collect())
            .collect()
    }

    /// The value the fake keeps for one of the app's items.
    pub fn item(&self, account: &str) -> Option<String> {
        std::fs::read_to_string(
            self.sec
                .join("items")
                .join(format!("com.shin315.quota-station.antigravity--{account}")),
        )
        .ok()
    }

    /// Makes the fake `security` fail one command (`add`, `delete`).
    pub fn fail_security(&self, command: &str) {
        std::fs::write(self.sec.join(format!("fail-{command}")), "").unwrap();
    }
}

/// A `security` that keeps generic passwords as files in `dir/items`, exits
/// 44 for a missing one as macOS's does, and records every run.
pub fn fake_security(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("items")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/security"),
        &format!(
            r#"for a in "$@"; do printf '%s\037' "$a" >> '{d}/calls'; done
printf '\n' >> '{d}/calls'
cmd="$1"; shift
svc=""; acct=""; value=""
while [ $# -gt 0 ]; do
  case "$1" in
    -s) svc="$2"; shift 2 ;;
    -a) acct="$2"; shift 2 ;;
    -w) if [ $# -ge 2 ]; then value="$2"; shift 2; else shift; fi ;;
    *) shift ;;
  esac
done
item='{d}/items/'"$svc--$acct"
missing() {{ echo "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain." >&2; exit 44; }}
case "$cmd" in
  add-generic-password)
    [ -e '{d}/fail-add' ] && {{ echo "security: SecKeychainItemCreateFromContent: User interaction is not allowed." >&2; exit 36; }}
    printf '%s' "$value" > "$item" ;;
  find-generic-password)
    [ -f "$item" ] || missing
    cat "$item"; echo ;;
  delete-generic-password)
    [ -e '{d}/fail-delete' ] && {{ echo "security: SecKeychainItemDelete: User interaction is not allowed." >&2; exit 36; }}
    [ -f "$item" ] || missing
    rm "$item" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}
