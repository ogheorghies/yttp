# 0020 · [feature] Export `build_query_string` as public API

## Goal

Export the internal `build_query_string` function so consumers like yurl can use it directly instead of reimplementing query string serialization.

## Approach

1. Make `build_query_string` public: `pub fn build_query_string(obj: &Map<String, Value>) -> String`
2. Also export the helper `append_query_to_url(url: &mut String, q: &Option<Value>)` that handles the `?`/`&` joining and empty-object no-op — this is the pattern both yttp's `parse_request` and yurl need.

## Deliverables

- `src/lib.rs`: make `build_query_string` public, add `append_query_to_url` convenience function
- Bump version to 0.0.4

## Acceptance criteria

- `yttp::build_query_string` is callable from external crates
- `yttp::append_query_to_url` appends `?key=val&...` or `&key=val&...` to a URL string, no-ops on `None` or empty object
- Existing tests pass, no behavior change
