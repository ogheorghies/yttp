# Delivery: Query parameters as structured `q:` key

## What was delivered

- `q:` key in yttp request schema, accepting an object of key-value pairs
- URL-encoded query string construction with automatic encoding
- Array values expand to repeated keys: `tags: [a, b]` → `tags=a&tags=b`
- Merges with existing URL query params (`?x=1` + `q: {y: 2}` → `?x=1&y=2`)
- Supports string, number, boolean, and null values
- 7 new tests covering all acceptance criteria
- README updated with `q:` reference documentation

## Files changed

- `src/lib.rs`: `parse_request` extracts `q:`, three new helpers (`encode_query_component`, `value_to_string`, `build_query_string`), 7 tests
- `README.md`: `q:` section in request reference, updated API table
