# yttp - "Better HTTP"

`yttp` is a Rust library providing a JSON/YAML façade for HTTP requests and responses.
It offers header shortcuts, smart auth, content-type-driven body encoding, and structured response formatting.

Used by [`yurl`](https://github.com/ogheorghies/yurl) — the CLI HTTP client built on `yttp`.

```rust
use yttp::{parse, parse_request, Request, Response, Status, format_response};

// Parse a "Better HTTP" request
let val = parse("{g: https://example.com, h: {a!: my-token}}")?;
let req = parse_request(&val)?;
// req.method == "GET", req.headers["Authorization"] == "Bearer my-token"

// Send with your preferred HTTP client, then format the response
let resp = Response {
    status: Status { line: "HTTP/1.1 200 OK".into(), version: "HTTP/1.1".into(), code: 200, text: "OK".into() },
    headers_raw: "content-type: application/json\r\n".into(),
    headers: /* from response */,
    body: br#"{"id": 1}"#.to_vec(),
};
let output = format_response(&resp);
// output["s"]["c"] == 200, output["b"]["id"] == 1
```

## Reference

Commented YAML schema by example.

### Request (input)

```yaml
# --- Method + URL ---
g: https://httpbin.org/get           # shortcuts: g p d (get post delete)
                                     # full: get post put delete patch head options trace
                                     # any capitalization accepted

# --- Request (input) headers ---
h:
  X-Request-Id: abc-123              # regular headers pass through as-is

  # key shortcuts — Authorization
  a!: my-token                       # → Authorization: Bearer my-token (bare token)
  a!: [user, pass]                   # → Authorization: Basic base64(user:pass)
  a!: Basic dXNlcjpwYXNz             # → passthrough (string with space = has scheme)
  a!: basic!user:pass                # → explicit prefix form: Basic base64(user:pass)
  a!: bearer!my-token                # → explicit prefix form: Bearer my-token

  # key shortcuts — Content-Type
  c!: f!                             # → Content-Type: application/x-www-form-urlencoded

  # value shortcuts — content types
  Accept: j!                         # → application/json          (also: json!)
  Accept: f!                         # → application/x-www-form..  (also: form!)
  Accept: m!                         # → multipart/form-data       (also: multi!)
  Accept: h!                         # → text/html                 (also: html!)
  Accept: t!                         # → text/plain                (also: text!)
  Accept: x!                         # → application/xml           (also: xml!)

  # value shortcuts — prefixes
  Accept: a!/json                    # → application/json
  Accept: t!/csv                     # → text/csv
  Accept: i!/png                     # → image/png

# --- Request (input) query params ---
q:                                   # merged into URL query string
  term: foo                          #   ?term=foo&limit=10
  limit: 10                          #   URL-encoded automatically
                                     #   arrays → repeated keys: tags: [a, b] → tags=a&tags=b
                                     #   appends to existing ?params if present

# --- Request (input) body ---
b:                                   # encoding depends on Content-Type:
  city: Berlin                       #   application/json (default, c!: j! is implied)
  lang: de                           #   c!: f! → form-urlencoded
                                     #   c!: m! → multipart (file:// values read from disk)
```

### Response (output)

Default structured format: `{s, h, b}`

```yaml
# --- Response (output) status ---
s: {v: HTTP/1.1, c: 200, t: OK}      # inline object (default via status_inline())
s: HTTP/1.1 200 OK                   # raw status line (via status.line)

# --- Response (output) headers ---
h:
  content-type: application/json
  server: gunicorn/19.9.0

# --- Response (output) body (via encode_body()) ---
# JSON response → preserved as structure:
b:
  city: Berlin
  lang: de
# UTF-8 text response → string (block scalar for multi-line):
b: |-
  <!doctype html>
  <html>
    <body>Hello</body>
  </html>
# binary response → base64 string:
b: "SGVsbG8gV29ybGQ=..."
```

## API

| Function | Description |
|---|---|
| `parse(s)` | Parse JSON or YAML string into `Value` |
| `parse_request(val)` | Parse request value, expand header shortcuts, merge `q:` into URL → `Request` |
| `parse_url(url)` | Parse URL into components → `UrlParts` |
| `encode_body(bytes)` | Smart body encoding: JSON → value, UTF-8 → string, binary → base64 |
| `status_inline(status)` | Format status as `{v, c, t}` object |
| `format_response(resp)` | Full response as `{s, h, b}` value |
| `headers_to_raw(headers)` | Headers map to raw HTTP string |
| `expand_headers(headers)` | Expand shortcut keys and values in-place |
| `build_query_string(obj)` | Build URL query string from key-value object |
| `append_query_to_url(url, q)` | Append `q:` params to URL string (no-op if `None`/empty) |

All functions that can fail return `yttp::Result<T>`.
