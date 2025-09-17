module.exports = function loadJSBundle(bundle) {
  // Don't insert the same script twice (e.g. if it was already in the HTML)
  let existingScripts = document.getElementsByTagName('script');
  let isCurrentBundle = function (script) {
    return script.src === bundle;
  };

  if ([].concat(existingScripts).some(isCurrentBundle)) {
    return;
  }

  // Request using XHR because it's synchronous and we can't use promises here
  // This has extremely poor performance because we're idle during this fetch, so we only use this so that the app won't crash
  const xhr = new XMLHttpRequest();
  xhr.open('GET', bundle, false);

  try {
    xhr.send();

    if (xhr.status === 200) {
      const script = document.createElement('script');
      script.type = 'text/javascript';
      script.text = xhr.responseText;

      // Execute the script synchronously
      document.head.appendChild(script);
    } else {
      throw new TypeError(
        `Failed to fetch dynamically imported module: ${bundle}. Status: ${xhr.status}`,
      );
    }
  } catch (e) {
    throw new TypeError(
      `Failed to fetch dynamically imported module: ${bundle}. Error: ${e.message}`,
    );
  }
};
