function sendAnalyticsEvent(detail) {
  const ev = new globalThis.CustomEvent('atlaspack:analytics', {detail});
  globalThis.dispatchEvent(ev);
}

module.exports = {sendAnalyticsEvent};
