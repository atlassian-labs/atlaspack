async function load(id) {
  // Global state maps the initial url to the completed task.
  // This ensures the same URL is used in subsequent imports
  if (!parcelRequire._retryState) parcelRequire._retryState = {};
  /** @type {Record<string, Promise<void>>} */
  const retryState = parcelRequire._retryState;

  // The number of retries before rethrowing the error
  const maxRetries = 6;

  // Resolve the request URL from the bundle ID
  const url = require('../bundle-manifest').resolve(id);

  // Wait for the user to go online before making a request
  if (!globalThis.navigator.onLine) {
    await new Promise((resolve) =>
      globalThis.addEventListener('online', resolve, {once: true}),
    );
  }

  // If the import has not run or is not currently running
  // then start the import retry task. Otherwise reuse the
  // existing result or wait for the current task to complete
  if (!retryState[url]) {
    retryState[url] = (async () => {
      // Try first request with normal import circuit
      try {
        // eslint-disable-next-line no-undef
        return await __parcel__import__(url);
      } catch {
        /**/
      }

      // Attempt to retry request
      for (let i = 1; i <= maxRetries; i++) {
        try {
          // Wait for an increasing delay time
          const jitter = Math.round(Math.random() * 100);
          const delay = Math.min(Math.pow(2, i), 8) * 1000;
          await new Promise((resolve) => setTimeout(resolve, delay + jitter));

          // Append the current time to the request URL
          // to ensure it has not been cached by the browser
          // eslint-disable-next-line no-undef
          return await __parcel__import__(`${url}?t=${Date.now()}`);
        } catch (error) {
          if (i === maxRetries) throw error;
          // Dispatch event for reporting
          const event = {detail: {target: url, attempt: i}};
          globalThis.dispatchEvent(
            new CustomEvent('atlaspack:import_retry', event),
          );
        }
      }
    })();
  }

  return retryState[url];
}

module.exports = load;
