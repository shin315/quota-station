// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A local HTTP server that answers by path and records every request: a
//! fake ChatGPT for Codex, or a fake Antigravity language server. A TLS
//! handshake is closed at once, as the Antigravity read tries HTTPS first.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// A request the fake answered.
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
}

pub struct FakeHttp {
    pub base: String,
    pub port: u16,
    answers: Arc<Mutex<BTreeMap<String, (u16, String)>>>,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl FakeHttp {
    pub fn start() -> FakeHttp {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let answers = Arc::new(Mutex::new(BTreeMap::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (a, r) = (answers.clone(), requests.clone());
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (a, r) = (a.clone(), r.clone());
                thread::spawn(move || serve(stream, &a, &r));
            }
        });
        FakeHttp { base: format!("http://127.0.0.1:{port}"), port, answers, requests }
    }

    /// `path` answers `status` with `body`; any other path is a 404.
    pub fn answer(&self, path: &str, status: u16, body: &str) {
        self.answers.lock().unwrap().insert(path.into(), (status, body.into()));
    }

    /// Every path answers `status` with `body`, until answered otherwise.
    pub fn answer_all(&self, status: u16, body: &str) {
        let mut answers = self.answers.lock().unwrap();
        for answer in answers.values_mut() {
            *answer = (status, body.into());
        }
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

fn serve(
    stream: TcpStream,
    answers: &Mutex<BTreeMap<String, (u16, String)>>,
    requests: &Mutex<Vec<Request>>,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    // 0x16 opens a TLS handshake: not for a plain HTTP server.
    if reader.fill_buf().map_or(true, |b| b.first() == Some(&0x16)) {
        return;
    }
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
    let request =
        Request { method, path, headers, body: String::from_utf8_lossy(&body).into_owned() };
    requests.lock().unwrap().push(request.clone());
    let (status, body) =
        answers.lock().unwrap().get(&request.path).cloned().unwrap_or((404, "{}".into()));
    let reply = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream = stream;
    let _ = stream.write_all(reply.as_bytes());
}
