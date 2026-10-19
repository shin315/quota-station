//! JSON read with the place of every value, so an edit replaces only the
//! bytes of one member and the rest of the file stays as its owner wrote
//! it. `serde_json` checks the text first and gives the error a person
//! reads; this parser only finds places in text `serde_json` accepted.

use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// From the first byte of the value to just past its last.
    pub span: Range<usize>,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Object(Vec<Member>),
    Array(Vec<Node>),
    Str(String),
    Num,
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub key: String,
    /// The key with its quotes.
    pub key_span: Range<usize>,
    pub value: Node,
}

/// Deeper nesting than any agent file has is refused.
const MAX_DEPTH: usize = 128;

/// Parses `text`, which must be one JSON value with only whitespace around.
pub fn parse(text: &str) -> Result<Node, String> {
    serde_json::from_str::<serde_json::Value>(text).map_err(|e| e.to_string())?;
    let mut p = Parser { s: text.as_bytes(), text, i: 0 };
    p.ws();
    let node = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err("text after the value".into());
    }
    Ok(node)
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at byte {}", c as char, self.i))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Node, String> {
        if depth > MAX_DEPTH {
            return Err("nested too deep".into());
        }
        let start = self.i;
        let kind = match self.s.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                let mut members = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                } else {
                    loop {
                        self.ws();
                        let key_start = self.i;
                        let key = self.string()?;
                        let key_span = key_start..self.i;
                        self.ws();
                        self.eat(b':')?;
                        self.ws();
                        let value = self.value(depth + 1)?;
                        members.push(Member { key, key_span, value });
                        self.ws();
                        match self.s.get(self.i) {
                            Some(b',') => self.i += 1,
                            Some(b'}') => {
                                self.i += 1;
                                break;
                            }
                            _ => return Err(format!("expected `,` or `}}` at byte {}", self.i)),
                        }
                    }
                }
                Kind::Object(members)
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                } else {
                    loop {
                        self.ws();
                        items.push(self.value(depth + 1)?);
                        self.ws();
                        match self.s.get(self.i) {
                            Some(b',') => self.i += 1,
                            Some(b']') => {
                                self.i += 1;
                                break;
                            }
                            _ => return Err(format!("expected `,` or `]` at byte {}", self.i)),
                        }
                    }
                }
                Kind::Array(items)
            }
            Some(b'"') => Kind::Str(self.string()?),
            Some(b't') => {
                self.word("true")?;
                Kind::Bool(true)
            }
            Some(b'f') => {
                self.word("false")?;
                Kind::Bool(false)
            }
            Some(b'n') => {
                self.word("null")?;
                Kind::Null
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                while self.i < self.s.len()
                    && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    self.i += 1;
                }
                Kind::Num
            }
            _ => return Err(format!("unexpected byte at {}", self.i)),
        };
        Ok(Node { span: start..self.i, kind })
    }

    fn word(&mut self, w: &str) -> Result<(), String> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(())
        } else {
            Err(format!("unexpected word at byte {}", self.i))
        }
    }

    /// A string, its quotes included; `serde_json` reads the escapes.
    fn string(&mut self) -> Result<String, String> {
        let start = self.i;
        self.eat(b'"')?;
        while self.i < self.s.len() {
            match self.s[self.i] {
                b'\\' => self.i += 2,
                b'"' => {
                    self.i += 1;
                    return serde_json::from_str(&self.text[start..self.i])
                        .map_err(|e| e.to_string());
                }
                _ => self.i += 1,
            }
        }
        Err("a string is not closed".into())
    }
}

impl Node {
    pub fn members(&self) -> &[Member] {
        match &self.kind {
            Kind::Object(m) => m,
            _ => &[],
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self.kind, Kind::Object(_))
    }

    /// The last member under `key`: JSON readers keep the last of a
    /// repeated key.
    pub fn get(&self, key: &str) -> Option<&Member> {
        self.members().iter().rev().find(|m| m.key == key)
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Kind::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// One replacement in the original text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub at: Range<usize>,
    pub text: String,
}

/// Makes the edits, which must not overlap.
pub fn apply(text: &str, mut edits: Vec<Edit>) -> String {
    let mut out = text.to_string();
    edits.sort_by_key(|e| std::cmp::Reverse(e.at.start));
    for e in edits {
        out.replace_range(e.at, &e.text);
    }
    out
}

/// The spaces before `at` on its line, when only spaces are there.
pub fn indent_at(text: &str, at: usize) -> Option<&str> {
    let start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let lead = &text[start..at];
    lead.chars().all(|c| c == ' ' || c == '\t').then_some(lead)
}

/// The edit that removes member `index` of `object`, with its comma.
pub fn remove_member(object: &Node, index: usize) -> Edit {
    let members = object.members();
    let m = &members[index];
    let at = if index + 1 < members.len() {
        // Up to the next key: its indentation stays and serves that key.
        m.key_span.start..members[index + 1].key_span.start
    } else if index > 0 {
        // From the end of the one before, so its comma goes too.
        members[index - 1].value.span.end..m.value.span.end
    } else {
        // The only member: the object becomes `{}`.
        object.span.start + 1..object.span.end - 1
    };
    Edit { at, text: String::new() }
}

/// The edit that adds `"key": value` as the last member of `object`.
/// `value` is already laid out for the member's indentation.
pub fn add_member(text: &str, object: &Node, key: &str, value: &str, unit: &str) -> Edit {
    let quoted = serde_json::to_string(key).expect("a string serializes");
    let members = object.members();
    match members.last() {
        Some(last) => {
            let indent = indent_at(text, last.key_span.start).unwrap_or(unit).to_string();
            Edit {
                at: last.value.span.end..last.value.span.end,
                text: format!(",\n{indent}{quoted}: {value}"),
            }
        }
        None => {
            let outer = indent_at(text, line_first(text, object.span.start)).unwrap_or("");
            Edit {
                at: object.span.start + 1..object.span.end - 1,
                text: format!("\n{outer}{unit}{quoted}: {value}\n{outer}"),
            }
        }
    }
}

/// The first non-space byte of the line `at` is on.
fn line_first(text: &str, at: usize) -> usize {
    let start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    start + text[start..].len() - text[start..].trim_start_matches([' ', '\t']).len()
}

/// The indentation step the file uses: the first member's indentation.
pub fn unit(text: &str, root: &Node) -> String {
    root.members()
        .first()
        .and_then(|m| indent_at(text, m.key_span.start))
        .filter(|s| !s.is_empty())
        .unwrap_or("  ")
        .to_string()
}

/// A JSON value laid out for a member at `indent`, with `unit` a step:
/// objects one member per line, arrays of plain values on one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Str(String),
    Bool(bool),
    Strs(Vec<String>),
    Obj(Vec<(String, Out)>),
    /// Text kept as the file wrote it.
    Raw(String),
}

pub fn layout(value: &Out, indent: &str, unit: &str) -> String {
    let q = |s: &str| serde_json::to_string(s).expect("a string serializes");
    match value {
        Out::Str(s) => q(s),
        Out::Bool(b) => b.to_string(),
        Out::Raw(s) => s.clone(),
        Out::Strs(items) => {
            format!("[{}]", items.iter().map(|s| q(s)).collect::<Vec<_>>().join(", "))
        }
        Out::Obj(members) if members.is_empty() => "{}".into(),
        Out::Obj(members) => {
            let inner = format!("{indent}{unit}");
            let lines: Vec<String> = members
                .iter()
                .map(|(k, v)| format!("{inner}{}: {}", q(k), layout(v, &inner, unit)))
                .collect();
            format!("{{\n{}\n{indent}}}", lines.join(",\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "{\n  \"mcpServers\": {\n    \"a\": {\n      \"command\": \"x\",\n      \"disabled\": true\n    },\n    \"b\": { \"serverUrl\": \"https://b.example.com\" }\n  },\n  \"other\": [1, 2.5e3, null, \"s\\\"q\"]\n}\n";

    #[test]
    fn places_match_the_text() {
        let root = parse(FILE).unwrap();
        let servers = &root.get("mcpServers").unwrap().value;
        let a = &servers.get("a").unwrap().value;
        let disabled = a.get("disabled").unwrap();
        assert_eq!(&FILE[disabled.value.span.clone()], "true");
        assert_eq!(&FILE[disabled.key_span.clone()], "\"disabled\"");
        let other = &root.get("other").unwrap().value;
        assert_eq!(&FILE[other.span.clone()], "[1, 2.5e3, null, \"s\\\"q\"]");
        let Kind::Array(items) = &other.kind else { panic!() };
        assert_eq!(items[3].as_str(), Some("s\"q"));
    }

    #[test]
    fn removing_and_adding_members_keeps_the_rest() {
        let root = parse(FILE).unwrap();
        let servers = &root.get("mcpServers").unwrap().value;
        let a = &servers.get("a").unwrap().value;
        // The last member of `a`.
        let out = apply(FILE, vec![remove_member(a, 1)]);
        assert!(out.contains("\"command\": \"x\"\n    },"), "{out}");
        // The first member of the servers.
        let out = apply(FILE, vec![remove_member(servers, 0)]);
        assert!(out.contains("\"mcpServers\": {\n    \"b\""), "{out}");
        serde_json::from_str::<serde_json::Value>(&out).unwrap();
        let value = layout(
            &Out::Obj(vec![
                ("command".into(), Out::Str("y".into())),
                ("args".into(), Out::Strs(vec!["-y".into()])),
            ]),
            "    ",
            "  ",
        );
        let out = apply(FILE, vec![add_member(FILE, servers, "c", &value, "  ")]);
        assert!(out.contains("},\n    \"c\": {\n      \"command\": \"y\",\n      \"args\": [\"-y\"]\n    }\n  },"), "{out}");
        serde_json::from_str::<serde_json::Value>(&out).unwrap();
    }

    #[test]
    fn adding_to_an_empty_object() {
        let text = "{\n  \"mcpServers\": {}\n}\n";
        let root = parse(text).unwrap();
        let servers = &root.get("mcpServers").unwrap().value;
        let out = apply(text, vec![add_member(text, servers, "a", "{}", "  ")]);
        assert_eq!(out, "{\n  \"mcpServers\": {\n    \"a\": {}\n  }\n}\n");
    }

    #[test]
    fn bad_json_is_refused_with_serde_s_message() {
        let err = parse("{\n  \"a\": [1,\n}").unwrap_err();
        assert!(err.contains("line 3"), "{err}");
    }
}
