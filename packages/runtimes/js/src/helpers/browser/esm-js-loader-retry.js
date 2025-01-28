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
          const result = await __parcel__import__(`${url}?t=${Date.now()}`);
          sendAnalyticsEvent('recovered', url, i);
          return result;
        } catch (error) {
          if (i === maxRetries) {
            sendAnalyticsEvent('failure', url, i);
            throw error;
          }
          sendAnalyticsEvent('progress', url, i);
        }
      }
    })();
  }

  return retryState[url];
}

function sendAnalyticsEvent(status, targetUrl, attempt) {
  require('./analytics/analytics.js').sendAnalyticsEvent({
    action: 'importRetry',
    attributes: {
      status,
      targetUrl,
      attempt,
    },
  });
}

module.exports = load;
