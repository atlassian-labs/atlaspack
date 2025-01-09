async function load(id) {
  const url = require('../bundle-manifest').resolve(id);

  const maxRetries = 6;

  for (let i = 1; i <= maxRetries; i++) {
    // Wait for the user to go online before making a request
    if (!globalThis.navigator.onLine) {
      await new Promise((resolve) =>
        globalThis.addEventListener('online', resolve, {once: true}),
      );
    }

    let requestUrl = url;
    if (i !== 0) {
      // Date ensures the client hasn't previously cached a failed request
      requestUrl = `${requestUrl}?retry=${i}:${Date.now()}`;
    }

    try {
      // eslint-disable-next-line no-undef
      return await __parcel__import__(requestUrl);
    } catch (error) {
      if (i === maxRetries) throw error;
      // Dispatch event for reporting
      window.dispatchEvent(
        new CustomEvent('atlaspack:import_retry', {
          detail: {
            target: url,
            attempt: i,
          },
        }),
      );
      const jitter = Math.round(Math.random() * 100);
      const delay = Math.min(Math.pow(2, i), 8) * 1000;

      await new Promise((resolve) => setTimeout(resolve, delay + jitter));
    }
  }
}

module.exports = load;
