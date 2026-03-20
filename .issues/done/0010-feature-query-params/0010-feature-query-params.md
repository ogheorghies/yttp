# 0010 · [feature] Query parameters as structured `q:` key

## Goal

Add a `q:` request key to yttp that represents URL query parameters as a structured object, instead of requiring them inline in the URL string.

## Approach

1. **Add `q:` key to the yttp request schema.** `q:` accepts an object of key-value pairs. Values can be strings, numbers, booleans, or arrays.

2. **URL construction.** When building the final URL, merge `q:` into the query string:
   - URL-encode keys and values automatically
   - Arrays become repeated keys: `tags: [a, b]` → `tags=a&tags=b`
   - If the URL already has query params (e.g. `?x=1`), append `q:` params after them

3. **Parsing.** `yttp::parse` extracts `q:` from the request object and returns it alongside method, URL, headers, body. The URL returned should have query params merged in.

4. **Update the spec/README** to document `q:` as a request key.

## Deliverables

- `src/lib.rs`: extract `q:` from parsed request, merge into URL as query string
- `README.md`: document `q:` key with examples
- Tests for: basic key-value, URL encoding of special characters, array expansion to repeated keys, merge with existing URL query params, missing `q:` (no-op)

## Acceptance criteria

- `{g: example.com/search, q: {term: foo, limit: 10}}` produces URL `https://example.com/search?term=foo&limit=10`
- `{g: example.com/search?x=1, q: {y: 2}}` produces URL `https://example.com/search?x=1&y=2`
- `{g: example.com/search, q: {tags: [a, b]}}` produces URL `https://example.com/search?tags=a&tags=b`
- `{g: example.com/search, q: {q: "hello world"}}` produces URL `https://example.com/search?q=hello%20world`
- Requests without `q:` are unchanged
- `cargo test` passes
