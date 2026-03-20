# Delivery: Export `build_query_string` as public API

## What was delivered

- `build_query_string` made public with doc comment
- `append_query_to_url` added as public convenience function
- `parse_request` refactored to use `append_query_to_url` (removes duplicated logic)
- README API table updated with both new exports
- Version bumped to 0.0.4

## Files changed

- `src/lib.rs`: visibility + new function + refactor
- `README.md`: API table
- `Cargo.toml`: version bump
