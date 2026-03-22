//! # yttp - "Better HTTP"
//!
//! A JSON/YAML façade for HTTP requests and responses.
//! Provides header shortcuts, smart auth, content-type-driven body encoding,
//! and structured response formatting.

mod shortcut;

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Map, Value};
use std::fmt;

pub use shortcut::expand_headers;

/// Error type for yttp operations.
#[derive(Debug)]
pub enum Error {
    Parse {
        msg: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    Request(String),
    Url(String),
}

impl Error {
    /// Create a parse error with position info.
    pub fn parse(msg: impl Into<String>, line: Option<usize>, column: Option<usize>) -> Self {
        Error::Parse { msg: msg.into(), line, column }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse { msg, line, column } => {
                write!(f, "parse error: {msg}")?;
                match (line, column) {
                    (Some(l), Some(c)) => write!(f, " (line {l}, column {c})"),
                    (Some(l), None) => write!(f, " (line {l})"),
                    (None, Some(c)) => write!(f, " (column {c})"),
                    (None, None) => Ok(()),
                }
            }
            Error::Request(msg) => write!(f, "request error: {msg}"),
            Error::Url(msg) => write!(f, "URL error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Parsed HTTP request, ready to send.
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Map<String, Value>,
    pub body: Option<Value>,
}

/// URL components.
pub struct UrlParts {
    pub scheme: String,
    pub host: String,
    pub port: String,
    pub path: String,
    pub query: String,
    pub fragment: String,
}

/// Structured status.
pub struct Status {
    pub line: String,
    pub version: String,
    pub code: u16,
    pub text: String,
}

/// Structured response.
pub struct Response {
    pub status: Status,
    pub headers_raw: String,
    pub headers: Map<String, Value>,
    pub body: Vec<u8>,
}

/// Parse a JSON/YAML string into a serde_json::Value.
///
/// Tries JSON first, then YAML. On failure, returns `Error::Parse` with
/// position info extracted from the serde error. For JSON-like input
/// (starts with `{`), the JSON parser's position is preferred since YAML
/// may parse partial JSON differently.
pub fn parse(s: &str) -> Result<Value> {
    match serde_json::from_str(s) {
        Ok(val) => return Ok(val),
        Err(json_err) => {
            match serde_yml::from_str::<Value>(s) {
                Ok(val) => return Ok(val),
                Err(yaml_err) => {
                    // Decide whether to show JSON or YAML error.
                    // If input looks like actual JSON (quoted keys), prefer JSON error.
                    // If input looks like YAML flow (unquoted keys), prefer YAML error.
                    let trimmed = s.trim_start();
                    let looks_like_json = trimmed.starts_with("{\"") || trimmed.starts_with("[");
                    if looks_like_json {
                        return Err(Error::parse(
                            format!("invalid JSON: {json_err}"),
                            Some(json_err.line()),
                            Some(json_err.column()),
                        ));
                    }
                    // YAML flow or block — use YAML error
                    let (line, col) = yaml_err.location().map_or(
                        (None, None),
                        |loc| (Some(loc.line()), Some(loc.column())),
                    );
                    return Err(Error::parse(
                        format!("invalid YAML: {yaml_err}"),
                        line,
                        col,
                    ));
                }
            }
        }
    }
}

/// Parse a request from a JSON/YAML value, expanding header shortcuts.
pub fn parse_request(val: &Value) -> Result<Request> {
    let obj = val
        .as_object()
        .ok_or_else(|| Error::Request("request must be a JSON/YAML object".into()))?;

    let mut method = None;
    let mut url = None;
    let mut headers = None;
    let mut body = None;
    let mut query = None;

    for (key, v) in obj {
        if let Some(m) = resolve_method(key) {
            method = Some(m.to_string());
            url = Some(
                v.as_str()
                    .ok_or_else(|| Error::Request(format!("URL for method '{key}' must be a string")))?
                    .to_string(),
            );
        } else {
            match key.to_lowercase().as_str() {
                "h" | "headers" => headers = Some(v.clone()),
                "b" | "body" => body = Some(v.clone()),
                "q" | "query" => query = Some(v.clone()),
                _ => {}
            }
        }
    }

    let mut header_map = headers
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    expand_headers(&mut header_map);

    let mut final_url = url
        .ok_or_else(|| Error::Request("no URL found".into()))?;

    append_query_to_url(&mut final_url, &query)?;

    Ok(Request {
        method: method
            .ok_or_else(|| Error::Request("no HTTP method found".into()))?,
        url: final_url,
        headers: header_map,
        body,
    })
}

/// Parse URL into components.
pub fn parse_url(url_str: &str) -> Result<UrlParts> {
    let parsed = url::Url::parse(url_str)
        .map_err(|e| Error::Url(format!("{e}")))?;
    Ok(UrlParts {
        scheme: parsed.scheme().to_string(),
        host: parsed.host_str().unwrap_or("").to_string(),
        port: parsed.port().map(|p| p.to_string()).unwrap_or_default(),
        path: parsed.path().trim_start_matches('/').to_string(),
        query: parsed.query().unwrap_or("").to_string(),
        fragment: parsed.fragment().unwrap_or("").to_string(),
    })
}

/// Encode response body for structured output: JSON → value, UTF-8 → string, binary → base64.
pub fn encode_body(bytes: &[u8]) -> Value {
    if let Ok(json_val) = serde_json::from_slice::<Value>(bytes) {
        return json_val;
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Value::String(s.to_string());
    }
    Value::String(STANDARD.encode(bytes))
}

/// Format status as an inline object {v, c, t}.
pub fn status_inline(status: &Status) -> Value {
    let mut m = Map::new();
    m.insert("v".to_string(), Value::String(status.version.clone()));
    m.insert("c".to_string(), Value::Number(status.code.into()));
    m.insert("t".to_string(), Value::String(status.text.clone()));
    Value::Object(m)
}

/// Format a full response as a structured value (s!, h, b).
pub fn format_response(resp: &Response) -> Value {
    let mut map = Map::new();
    map.insert("s".to_string(), status_inline(&resp.status));
    map.insert("h".to_string(), Value::Object(resp.headers.clone()));
    map.insert("b".to_string(), encode_body(&resp.body));
    Value::Object(map)
}

/// Build request headers as raw HTTP string.
pub fn headers_to_raw(headers: &Map<String, Value>) -> String {
    let mut raw = String::new();
    for (k, v) in headers {
        if let Some(s) = v.as_str() {
            raw.push_str(&format!("{k}: {s}\r\n"));
        }
    }
    raw
}

fn encode_query_component(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

/// Build a URL query string from an object of key-value pairs.
///
/// Values are URL-encoded. Arrays expand to repeated keys:
/// `tags: [a, b]` → `tags=a&tags=b`.
pub fn build_query_string(obj: &Map<String, Value>) -> String {
    let mut pairs = Vec::new();
    for (k, v) in obj {
        let key = encode_query_component(k);
        match v {
            Value::Array(arr) => {
                for item in arr {
                    pairs.push(format!("{}={}", key, encode_query_component(&value_to_string(item))));
                }
            }
            _ => {
                pairs.push(format!("{}={}", key, encode_query_component(&value_to_string(v))));
            }
        }
    }
    pairs.join("&")
}

/// Append query parameters from a `q:` value to a URL string.
///
/// No-ops if `q` is `None` or an empty object. Appends with `&` if the URL
/// already has a `?`, otherwise adds `?`.
pub fn append_query_to_url(url: &mut String, q: &Option<Value>) -> Result<()> {
    let Some(q) = q else { return Ok(()) };
    let obj = q
        .as_object()
        .ok_or_else(|| Error::Request("q must be an object".into()))?;
    if obj.is_empty() {
        return Ok(());
    }
    let qs = build_query_string(obj);
    if url.contains('?') {
        url.push('&');
    } else {
        url.push('?');
    }
    url.push_str(&qs);
    Ok(())
}

pub fn resolve_method(key: &str) -> Option<&'static str> {
    match key.to_lowercase().as_str() {
        "get" | "g" => Some("GET"),
        "post" | "p" => Some("POST"),
        "put" => Some("PUT"),
        "delete" | "d" => Some("DELETE"),
        "patch" => Some("PATCH"),
        "head" => Some("HEAD"),
        "options" => Some("OPTIONS"),
        "trace" => Some("TRACE"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse ---

    #[test]
    fn parse_json() {
        let val = parse(r#"{"g": "https://example.com"}"#).unwrap();
        assert_eq!(val["g"], "https://example.com");
    }

    #[test]
    fn parse_yaml_block() {
        let val = parse("g: https://example.com\nh:\n  Accept: j!\n").unwrap();
        assert_eq!(val["g"], "https://example.com");
        assert_eq!(val["h"]["Accept"], "j!");
    }

    #[test]
    fn parse_yaml_flow() {
        let val = parse("{g: https://example.com, h: {Accept: j!}}").unwrap();
        assert_eq!(val["g"], "https://example.com");
    }

    #[test]
    fn parse_invalid() {
        assert!(parse("{{invalid}}").is_err());
    }

    #[test]
    fn parse_yaml_flow_with_explicit_null() {
        let val = parse("{g: google.com, adad: null}").unwrap();
        assert_eq!(val["g"], "google.com");
        assert!(val["adad"].is_null());
    }

    #[test]
    fn parse_yaml_flow_error_not_json_error() {
        // {g: google.com, adad:} is YAML flow style — if it fails,
        // the error should be a YAML error, not "invalid JSON"
        let result = parse("{g: google.com, adad:}");
        if let Err(e) = &result {
            let msg = format!("{e}");
            assert!(!msg.contains("invalid JSON"), "should show YAML error for YAML input: {msg}");
        }
    }

    #[test]
    fn parse_error_json_has_position() {
        // Use actual JSON syntax (quoted keys) to trigger JSON error path
        let err = parse(r#"{"g": "broken", "b": {}"#).unwrap_err();
        match err {
            Error::Parse { line, column, msg, .. } => {
                assert!(line.is_some(), "should have line");
                assert!(column.is_some(), "should have column");
                assert!(msg.contains("invalid JSON"), "msg: {msg}");
            }
            _ => panic!("expected Parse error"),
        }
    }

    #[test]
    fn parse_error_yaml_has_position() {
        let err = parse("g: [\nunclosed").unwrap_err();
        match err {
            Error::Parse { msg, .. } => {
                assert!(msg.contains("invalid YAML"), "msg: {msg}");
            }
            _ => panic!("expected Parse error"),
        }
    }

    #[test]
    fn parse_error_display_includes_position() {
        let err = parse("{broken").unwrap_err();
        let display = format!("{err}");
        assert!(display.contains("parse error:"), "display: {display}");
        assert!(display.contains("line"), "should include position: {display}");
    }

    // --- parse_request ---

    #[test]
    fn parse_request_get() {
        let val = parse("{g: https://example.com}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert!(req.body.is_none());
    }

    #[test]
    fn parse_request_post_with_body() {
        let val = parse(r#"{"p": "https://example.com", "b": {"key": "val"}}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body.unwrap()["key"], "val");
    }

    #[test]
    fn parse_request_method_case_insensitive() {
        let val = parse(r#"{"GET": "https://example.com"}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.method, "GET");
    }

    #[test]
    fn parse_request_no_method() {
        let val = parse(r#"{"h": {"Accept": "j!"}}"#).unwrap();
        assert!(parse_request(&val).is_err());
    }

    #[test]
    fn parse_request_not_object() {
        let val = Value::String("not an object".into());
        assert!(parse_request(&val).is_err());
    }

    #[test]
    fn parse_request_all_methods() {
        for (short, full) in &[
            ("g", "GET"),
            ("p", "POST"),
            ("d", "DELETE"),
            ("put", "PUT"),
            ("patch", "PATCH"),
            ("head", "HEAD"),
            ("options", "OPTIONS"),
            ("trace", "TRACE"),
        ] {
            let val = parse(&format!("{{{short}: https://example.com}}")).unwrap();
            let req = parse_request(&val).unwrap();
            assert_eq!(req.method, *full);
        }
    }

    // --- header shortcuts ---

    #[test]
    fn header_bearer_bare_token() {
        let val = parse("{g: https://example.com, h: {a!: my-token}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.headers["Authorization"], "Bearer my-token");
    }

    #[test]
    fn header_bearer_explicit() {
        let val = parse("{g: https://example.com, h: {a!: bearer!tok}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.headers["Authorization"], "Bearer tok");
    }

    #[test]
    fn header_basic_array() {
        let val = parse(r#"{"g": "https://example.com", "h": {"a!": ["user", "pass"]}}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.headers["Authorization"], "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn header_basic_explicit() {
        let val = parse("{g: https://example.com, h: {a!: basic!user:pass}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.headers["Authorization"], "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn header_auth_scheme_passthrough() {
        let val = parse("{g: https://example.com, h: {a!: Digest abc123}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.headers["Authorization"], "Digest abc123");
    }

    #[test]
    fn header_content_type_shortcut() {
        let val = parse("{g: https://example.com, h: {c!: f!}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(
            req.headers["Content-Type"],
            "application/x-www-form-urlencoded"
        );
    }

    #[test]
    fn header_value_shortcuts() {
        let cases = vec![
            ("j!", "application/json"),
            ("json!", "application/json"),
            ("f!", "application/x-www-form-urlencoded"),
            ("m!", "multipart/form-data"),
            ("h!", "text/html"),
            ("t!", "text/plain"),
            ("x!", "application/xml"),
        ];
        for (shortcut, expected) in cases {
            let val =
                parse(&format!("{{g: https://example.com, h: {{Accept: {shortcut}}}}}")).unwrap();
            let req = parse_request(&val).unwrap();
            assert_eq!(req.headers["Accept"], expected, "shortcut {shortcut}");
        }
    }

    #[test]
    fn header_prefix_shortcuts() {
        let cases = vec![
            ("a!/json", "application/json"),
            ("t!/csv", "text/csv"),
            ("i!/png", "image/png"),
        ];
        for (shortcut, expected) in cases {
            let val =
                parse(&format!("{{g: https://example.com, h: {{Accept: {shortcut}}}}}")).unwrap();
            let req = parse_request(&val).unwrap();
            assert_eq!(req.headers["Accept"], expected, "prefix {shortcut}");
        }
    }

    // --- query params ---

    #[test]
    fn query_basic() {
        let val = parse("{g: https://example.com/search, q: {term: foo, limit: 10}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com/search?term=foo&limit=10");
    }

    #[test]
    fn query_merge_existing() {
        let val = parse("{g: https://example.com/search?x=1, q: {y: 2}}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com/search?x=1&y=2");
    }

    #[test]
    fn query_array_repeated_keys() {
        let val = parse(r#"{"g": "https://example.com/search", "q": {"tags": ["a", "b"]}}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com/search?tags=a&tags=b");
    }

    #[test]
    fn query_url_encoding() {
        let val = parse(r#"{"g": "https://example.com/search", "q": {"q": "hello world"}}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com/search?q=hello+world");
    }

    #[test]
    fn query_absent_noop() {
        let val = parse("{g: https://example.com}").unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn query_empty_noop() {
        let val = parse(r#"{"g": "https://example.com", "q": {}}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn query_boolean_value() {
        let val = parse(r#"{"g": "https://example.com", "q": {"active": true}}"#).unwrap();
        let req = parse_request(&val).unwrap();
        assert_eq!(req.url, "https://example.com?active=true");
    }

    // --- parse_url ---

    #[test]
    fn parse_url_parts() {
        let parts = parse_url("https://example.com:8080/api/items?q=test#section").unwrap();
        assert_eq!(parts.scheme, "https");
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.port, "8080");
        assert_eq!(parts.path, "api/items");
        assert_eq!(parts.query, "q=test");
        assert_eq!(parts.fragment, "section");
    }

    #[test]
    fn parse_url_defaults() {
        let parts = parse_url("https://example.com/path").unwrap();
        assert_eq!(parts.port, "");
        assert_eq!(parts.query, "");
        assert_eq!(parts.fragment, "");
    }

    #[test]
    fn parse_url_invalid() {
        assert!(parse_url("not a url").is_err());
    }

    // --- encode_body ---

    #[test]
    fn encode_body_json() {
        let body = encode_body(b"[1, 2, 3]");
        assert!(body.is_array());
        assert_eq!(body[0], 1);
    }

    #[test]
    fn encode_body_json_object() {
        let body = encode_body(br#"{"key": "val"}"#);
        assert!(body.is_object());
        assert_eq!(body["key"], "val");
    }

    #[test]
    fn encode_body_utf8() {
        let body = encode_body(b"hello world");
        assert_eq!(body, "hello world");
    }

    #[test]
    fn encode_body_binary() {
        let bytes = vec![0xff, 0xfe, 0x00, 0x01];
        let body = encode_body(&bytes);
        assert!(body.is_string());
        let s = body.as_str().unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(s)
                .unwrap(),
            bytes
        );
    }

    // --- status_inline ---

    #[test]
    fn status_inline_format() {
        let status = Status {
            line: "HTTP/1.1 200 OK".to_string(),
            version: "HTTP/1.1".to_string(),
            code: 200,
            text: "OK".to_string(),
        };
        let val = status_inline(&status);
        assert_eq!(val["v"], "HTTP/1.1");
        assert_eq!(val["c"], 200);
        assert_eq!(val["t"], "OK");
    }

    // --- format_response ---

    #[test]
    fn format_response_structure() {
        let resp = Response {
            status: Status {
                line: "HTTP/1.1 200 OK".to_string(),
                version: "HTTP/1.1".to_string(),
                code: 200,
                text: "OK".to_string(),
            },
            headers_raw: "content-type: application/json\r\n".to_string(),
            headers: {
                let mut m = Map::new();
                m.insert(
                    "content-type".to_string(),
                    Value::String("application/json".to_string()),
                );
                m
            },
            body: br#"{"id": 1}"#.to_vec(),
        };
        let val = format_response(&resp);
        assert_eq!(val["s"]["c"], 200);
        assert_eq!(val["h"]["content-type"], "application/json");
        assert_eq!(val["b"]["id"], 1);
    }

    // --- headers_to_raw ---

    #[test]
    fn headers_to_raw_format() {
        let mut headers = Map::new();
        headers.insert(
            "Accept".to_string(),
            Value::String("application/json".to_string()),
        );
        headers.insert(
            "Host".to_string(),
            Value::String("example.com".to_string()),
        );
        let raw = headers_to_raw(&headers);
        assert!(raw.contains("Accept: application/json\r\n"));
        assert!(raw.contains("Host: example.com\r\n"));
    }
}
