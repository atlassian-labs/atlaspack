---
'@atlaspack/transformer-tokens': minor
'@atlaspack/transformer-js': minor
'@atlaspack/core': minor
'@atlaspack/rust': minor
---

Set `SyntaxError` as the diagnostic name for parse/syntax errors from the JS and Tokens transformers.

Parse and syntax errors from the JS Transformer and Tokens Transformer now set `diagnostic.name` to `"SyntaxError"`, so consumers can reliably detect syntax/parse failures (e.g. for reporting or error handling). The Rust diagnostic type and NAPI `JsDiagnostic` include an optional `name` field; `error_buffer_to_diagnostics` sets it to `"SyntaxError"` for SWC parse errors, and both transformers pass it through to the thrown diagnostic. Integration tests assert that the first diagnostic has `name === 'SyntaxError'` for tokens and JS parse-error cases.
