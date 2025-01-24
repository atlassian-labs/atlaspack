const EVENT_NAME = 'atlaspack:analytics';

function sendAnalyticsEvent(detail) {
  const ev = new globalThis.CustomEvent(EVENT_NAME, {detail});
  globalThis.dispatchEvent(ev);
}

module.exports = {sendAnalyticsEvent};
