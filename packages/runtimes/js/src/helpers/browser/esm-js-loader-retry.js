async function load(id) {
  // Resolve the request URL from the bundle ID
  const url = require('../bundle-manifest').resolve(id);

  // Global state maps the initial url to the completed task.
  // This ensures the same URL is used in subsequent imports
  if (!parcelRequire._retryState) parcelRequire._retryState = {};
  const retryState = parcelRequire._retryState;

  // The number of retries before rethrowing the error
  const maxRetries = 6;

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
      // Attempt to retry request
      for (let i = 0; i <= maxRetries; i++) {
        if (await checkUrlLoads(url)) {
          break;
        }

        // Dispatch event for reporting
        const event = {detail: {target: url, attempt: i}};
        globalThis.dispatchEvent(
          new CustomEvent('atlaspack:import_retry', event),
        );

        // Wait for an increasing delay time
        const jitter = Math.round(Math.random() * 100);
        const delay = Math.min(Math.pow(2, i), 8) * 1000;
        await new Promise((resolve) => setTimeout(resolve, delay + jitter));
      }

      // eslint-disable-next-line no-undef
      return __parcel__import__(url);
    })();
  }

  return retryState[url];
}

// Check the target URL can be loaded via a preload tag
async function checkUrlLoads(href) {
  const link = globalThis.document.createElement('link');
  link.rel = 'preload';
  link.crossOrigin = true;
  link.as = 'script';
  link.href = href;

  const onload = new Promise((res) => (link.onload = () => res(true)));
  const onerror = new Promise((res) => (link.onerror = () => res(false)));

  globalThis.document.head.appendChild(link);
  const result = await Promise.race([onload, onerror]);
  globalThis.document.head.removeChild(link);

  return result;
}

module.exports = load;
