// Ported from Cockpit Tools `src-tauri/src/modules/oauth_server.rs` (1.3.70,
// commit 244b957, CC BY-NC-SA 4.0); changes: a thread and a channel instead
// of tokio and Tauri events; the flow is a value held by its caller instead
// of a global; typed errors; the pages the browser sees are in Vietnamese.
// The listener, the redirect URI, the state check, the request limits, the
// 10-minute wait and the pasted-URL rules are Cockpit's.

//! The loopback server Google sends the browser back to after the consent
//! screen, and the pasted callback URL for a browser that cannot reach it.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use url::Url;

use super::client::OAuthClient;
use super::oauth::{self, Endpoints};

pub const CALLBACK_PATH: &str = "/oauth-callback";
const MAX_HTTP_REQUEST_BYTES: usize = 32 * 1024;
const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a login waits for the browser to come back.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// How often the listener looks for a connection or a stop.
const POLL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackError {
    NoCode,
    NoState,
    /// The callback's `state` is not this login's: not ours, or stale.
    StateMismatch,
    /// The browser's request could not be read or parsed.
    BadRequest(String),
    /// A pasted URL that is empty, not a URL, or not the callback path.
    BadUrl(String),
    /// A callback already ended this login.
    AlreadyHandled,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallbackError::NoCode => write!(f, "the callback has no authorization code"),
            CallbackError::NoState => write!(f, "the callback has no state"),
            CallbackError::StateMismatch => write!(f, "the callback's state does not match"),
            CallbackError::BadRequest(e) => write!(f, "bad callback request: {e}"),
            CallbackError::BadUrl(e) => write!(f, "bad callback URL: {e}"),
            CallbackError::AlreadyHandled => write!(f, "the callback was already handled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowError {
    /// No local port to listen on.
    Bind(String),
    Callback(CallbackError),
    Cancelled,
    /// No callback within the wait.
    Timeout,
}

impl std::fmt::Display for FlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowError::Bind(e) => write!(f, "cannot listen on a local port: {e}"),
            FlowError::Callback(e) => e.fmt(f),
            FlowError::Cancelled => write!(f, "the login was cancelled"),
            FlowError::Timeout => {
                write!(f, "no callback within {} minutes", LOGIN_TIMEOUT.as_secs() / 60)
            }
        }
    }
}

type Outcome = Result<String, FlowError>;

struct Shared {
    auth_url: String,
    redirect_uri: String,
    expected_state: String,
    /// Taken by whichever comes first: the callback, a pasted URL, a cancel.
    tx: Mutex<Option<mpsc::Sender<Outcome>>>,
    stop: AtomicBool,
}

impl Shared {
    fn finish(&self, outcome: Outcome) -> bool {
        let sender = self.tx.lock().unwrap_or_else(|e| e.into_inner()).take();
        match sender {
            Some(tx) => {
                let _ = tx.send(outcome);
                true
            }
            None => false,
        }
    }
}

/// What other threads use while a login waits: the URL to open, a pasted
/// callback, cancelling.
#[derive(Clone)]
pub struct Handle(Arc<Shared>);

impl Handle {
    pub fn auth_url(&self) -> &str {
        &self.0.auth_url
    }

    pub fn redirect_uri(&self) -> &str {
        &self.0.redirect_uri
    }

    /// Ends the wait with the code in a callback URL the owner pasted, after
    /// the same checks as a callback that arrives. A URL that fails them
    /// leaves the login waiting.
    pub fn submit_callback_url(&self, callback_url: &str) -> Result<(), CallbackError> {
        let parsed = parse_manual_callback_url(callback_url, &self.0.redirect_uri)?;
        if parsed.path() != CALLBACK_PATH {
            return Err(CallbackError::BadUrl(format!("the path must be {CALLBACK_PATH}")));
        }

        let code = extract_code_from_callback_url(&parsed, &self.0.expected_state)?;
        if self.0.finish(Ok(code)) {
            Ok(())
        } else {
            Err(CallbackError::AlreadyHandled)
        }
    }

    /// Stops listening and ends the wait.
    pub fn cancel(&self) {
        self.0.stop.store(true, Ordering::SeqCst);
        self.0.finish(Err(FlowError::Cancelled));
    }
}

/// A login listening for its callback.
pub struct Flow {
    handle: Handle,
    rx: mpsc::Receiver<Outcome>,
    client: OAuthClient,
}

impl Flow {
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    pub fn client(&self) -> &OAuthClient {
        &self.client
    }

    /// Waits for the code. Returns it with the redirect URI the exchange
    /// must repeat. The listener stops either way, when the flow drops.
    pub fn wait(self, timeout: Duration) -> Result<(String, String), FlowError> {
        let redirect_uri = self.handle.0.redirect_uri.clone();
        match self.rx.recv_timeout(timeout) {
            Ok(Ok(code)) => Ok((code, redirect_uri)),
            Ok(Err(e)) => Err(e),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(FlowError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(FlowError::Cancelled),
        }
    }
}

/// A login dropped without waiting stops listening too.
impl Drop for Flow {
    fn drop(&mut self) {
        self.handle.cancel();
    }
}

/// Listens on a random loopback port and builds the consent URL for it, with
/// a fresh `state`. Nothing is opened yet.
pub fn prepare(endpoints: &Endpoints, client: &OAuthClient) -> Result<Flow, FlowError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| FlowError::Bind(e.to_string()))?;
    let port = listener.local_addr().map_err(|e| FlowError::Bind(e.to_string()))?.port();
    listener.set_nonblocking(true).map_err(|e| FlowError::Bind(e.to_string()))?;

    let redirect_uri = format!("http://localhost:{}/oauth-callback", port);
    let state_token = uuid::Uuid::new_v4().to_string();
    let auth_url = oauth::get_auth_url(endpoints, client, &redirect_uri, Some(&state_token));

    let (tx, rx) = mpsc::channel();
    let shared = Arc::new(Shared {
        auth_url,
        redirect_uri,
        expected_state: state_token,
        tx: Mutex::new(Some(tx)),
        stop: AtomicBool::new(false),
    });

    let server = shared.clone();
    thread::spawn(move || {
        while !server.stop.load(Ordering::SeqCst) {
            let mut stream = match listener.accept() {
                Ok((stream, _)) => stream,
                Err(_) => {
                    thread::sleep(POLL);
                    continue;
                }
            };
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(REQUEST_READ_TIMEOUT));
            if let Some(result) =
                process_callback_request(&mut stream, port, &server.expected_state)
            {
                server.finish(result.map_err(FlowError::Callback));
                break;
            }
        }
    });

    Ok(Flow { handle: Handle(shared), rx, client: client.clone() })
}

const PAGE_STYLE: &str = "font-family: -apple-system, sans-serif; text-align: center; padding: 50px; background: #0d1117; color: #fff;";

fn oauth_success_html() -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
    <html>\
    <body style='{PAGE_STYLE}'>\
        <h1 style='color: #4ade80;'>✅ Đã đăng nhập</h1>\
        <p>Bạn có thể đóng cửa sổ này và quay lại Quota Station.</p>\
        <script>setTimeout(function() {{ window.close(); }}, 2000);</script>\
    </body>\
    </html>"
    )
}

fn oauth_fail_html(message: &str) -> String {
    format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
    <html>\
    <body style='{PAGE_STYLE}'>\
        <h1 style='color: #f87171;'>❌ Chưa đăng nhập được</h1>\
        <p>{}</p>\
    </body>\
    </html>",
        message
    )
}

fn oauth_not_found_response() -> &'static str {
    "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nNot Found"
}

fn oauth_options_response() -> &'static str {
    "HTTP/1.1 200 OK\r\n\
    Access-Control-Allow-Origin: *\r\n\
    Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
    Access-Control-Allow-Headers: Content-Type\r\n\
    Content-Length: 0\r\n\r\n"
}

fn extract_code_from_callback_url(
    callback_url: &Url,
    expected_state: &str,
) -> Result<String, CallbackError> {
    let mut code = None;
    let mut state = None;
    for (key, value) in callback_url.query_pairs() {
        match key.as_ref() {
            "code" if code.is_none() => code = Some(value.into_owned()),
            "state" if state.is_none() => state = Some(value.into_owned()),
            _ => {}
        }
    }

    let Some(code) = code.filter(|value| !value.trim().is_empty()) else {
        return Err(CallbackError::NoCode);
    };

    let Some(state) = state.filter(|value| !value.trim().is_empty()) else {
        return Err(CallbackError::NoState);
    };

    if state != expected_state {
        return Err(CallbackError::StateMismatch);
    }

    Ok(code)
}

fn parse_manual_callback_url(
    raw_callback_url: &str,
    redirect_uri: &str,
) -> Result<Url, CallbackError> {
    let bad = |e: url::ParseError| CallbackError::BadUrl(e.to_string());
    let trimmed = raw_callback_url.trim();
    if trimmed.is_empty() {
        return Err(CallbackError::BadUrl("empty".into()));
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Url::parse(trimmed).map_err(bad);
    }

    let redirect = Url::parse(redirect_uri).map_err(bad)?;
    let host = redirect.host_str().ok_or_else(|| CallbackError::BadUrl("no host".into()))?;
    let origin = match redirect.port() {
        Some(port) => format!("{}://{}:{}", redirect.scheme(), host, port),
        None => format!("{}://{}", redirect.scheme(), host),
    };

    if trimmed.starts_with('/') {
        return Url::parse(format!("{}{}", origin, trimmed).as_str()).map_err(bad);
    }

    Url::parse(format!("{}{}?{}", origin, CALLBACK_PATH, trimmed.trim_start_matches('?')).as_str())
        .map_err(bad)
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0u8; 2048];

    loop {
        let bytes_read = match stream.read(&mut chunk) {
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err("timed out reading the callback request".into())
            }
            Err(e) => return Err(format!("cannot read the callback request: {e}")),
        };

        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n")
            || buffer.len() >= MAX_HTTP_REQUEST_BYTES
        {
            break;
        }
    }

    if buffer.is_empty() {
        return Err("empty callback request".into());
    }

    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn parse_request_target(request: &str) -> Result<(String, String), String> {
    let request_line = request.lines().next().ok_or("no request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("no method")?;
    let target = parts.next().ok_or("no target")?;

    Ok((method.to_string(), target.to_string()))
}

fn respond(stream: &mut TcpStream, response: &str) {
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// One request to the listener. `None` keeps listening (a preflight, another
/// path); anything else ends the login.
fn process_callback_request(
    stream: &mut TcpStream,
    port: u16,
    expected_state: &str,
) -> Option<Result<String, CallbackError>> {
    let request = match read_http_request(stream) {
        Ok(request) => request,
        Err(err) => {
            respond(stream, &oauth_fail_html("Không đọc được yêu cầu. Quay lại app và thử lại."));
            return Some(Err(CallbackError::BadRequest(err)));
        }
    };

    let (method, target) = match parse_request_target(&request) {
        Ok(parsed) => parsed,
        Err(err) => {
            respond(stream, &oauth_fail_html("Yêu cầu không đúng dạng. Quay lại app và thử lại."));
            return Some(Err(CallbackError::BadRequest(err)));
        }
    };

    if method.eq_ignore_ascii_case("OPTIONS") {
        respond(stream, oauth_options_response());
        return None;
    }

    let callback_url = match if target.starts_with("http://") || target.starts_with("https://") {
        Url::parse(&target)
    } else {
        Url::parse(&format!("http://localhost:{}{}", port, target))
    } {
        Ok(url) => url,
        Err(_) => {
            respond(stream, &oauth_fail_html("Không đọc được URL. Quay lại app và thử lại."));
            return Some(Err(CallbackError::BadRequest("unparsable callback URL".into())));
        }
    };

    if callback_url.path() != CALLBACK_PATH {
        respond(stream, oauth_not_found_response());
        return None;
    }

    let code = match extract_code_from_callback_url(&callback_url, expected_state) {
        Ok(code) => code,
        Err(err) => {
            let page = match err {
                CallbackError::NoCode => "Google không gửi mã đăng nhập. Quay lại app và thử lại.",
                CallbackError::NoState => "Thiếu mã kiểm tra (state). Quay lại app và thử lại.",
                _ => "Mã kiểm tra (state) không khớp. Quay lại app và bắt đầu đăng nhập lại.",
            };
            respond(stream, &oauth_fail_html(page));
            return Some(Err(err));
        }
    };

    respond(stream, &oauth_success_html());

    Some(Ok(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pasted_callback_may_be_a_url_a_path_or_a_query() {
        let redirect = "http://localhost:4321/oauth-callback";
        for raw in [
            "http://localhost:4321/oauth-callback?code=c&state=s",
            "/oauth-callback?code=c&state=s",
            "?code=c&state=s",
            "code=c&state=s",
        ] {
            let url = parse_manual_callback_url(raw, redirect).unwrap();
            assert_eq!(url.path(), CALLBACK_PATH, "{raw}");
            assert_eq!(extract_code_from_callback_url(&url, "s").unwrap(), "c", "{raw}");
        }
        assert!(parse_manual_callback_url("  ", redirect).is_err());
        let url = parse_manual_callback_url("code=c&state=other", redirect).unwrap();
        assert_eq!(extract_code_from_callback_url(&url, "s"), Err(CallbackError::StateMismatch));
        let url = parse_manual_callback_url("state=s", redirect).unwrap();
        assert_eq!(extract_code_from_callback_url(&url, "s"), Err(CallbackError::NoCode));
        let url = parse_manual_callback_url("code=c", redirect).unwrap();
        assert_eq!(extract_code_from_callback_url(&url, "s"), Err(CallbackError::NoState));
    }
}
