# Useful tools for debugging tests

## Environment variables

### `ATLASPACK_MOCHA_HANG_DEBUG`

Enable debug mode for Mocha test hangs.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_MOCHA_HANG_DEBUG=true yarn test:integration` or `ATLASPACK_MOCHA_HANG_DEBUG=true yarn test:js:unit`

When enabled, this environment variable runs [`why-is-node-running`](https://github.com/mafintosh/why-is-node-running) for Mocha tests. If the tests complete but Mocha appears to have hung and won't exit you can send the `SIGHUP` signal to the process (see the command and PID at the top of the Mocha
output). `why-is-node-running` will show what is holding on to open handles. Most of the time this is a worker farm that hasn't had `end()` called on it.
