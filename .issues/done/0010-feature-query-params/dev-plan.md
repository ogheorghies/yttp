# Dev Plan: Query parameters as structured `q:` key

## Approach

1. Add `q:` / `query` key extraction in `parse_request` alongside `h` and `b`
2. Add `build_query_string` helper that URL-encodes key-value pairs, expanding arrays to repeated keys
3. Merge query string into URL (append with `&` if `?` exists, else `?`)
4. Add tests covering: basic params, merge with existing, array expansion, URL encoding, absent/empty q, boolean values
5. Update README reference section with `q:` documentation
