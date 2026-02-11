API and HTTP server for `@atlaspack/inspector`.

Uses:

- `express`
- `pino`
- `@atlaspack/core`
- `@atlaspack/query`
- `@atlaspack/graph`

Exposes data from the `atlaspack` cache through a HTTP API.

Serves the front-end when in production mode. The front-end is bundled and published
with this package.
