// Based on _.escape https://github.com/lodash/lodash/blob/master/escape.js
const reUnescapedHtml = /[&<>"']/g;
const reHasUnescapedHtml = RegExp(reUnescapedHtml.source);

const htmlEscapes = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
} as const;

export function escapeHTML(s: string): string {
  if (reHasUnescapedHtml.test(s)) {
    return s.replace(reUnescapedHtml, (c) => htmlEscapes[c]);
  }

  return s;
}
