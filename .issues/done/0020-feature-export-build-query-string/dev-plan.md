# Dev Plan: Export `build_query_string` as public API

1. Make `build_query_string` public with doc comment
2. Add `append_query_to_url` convenience function that handles `?`/`&` joining and None/empty no-op
3. Refactor `parse_request` to use `append_query_to_url` internally
4. Update README API table
5. Bump version to 0.0.4
