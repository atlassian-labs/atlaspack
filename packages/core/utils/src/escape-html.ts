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
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type '{ readonly '&': "&amp;"; readonly '<': "&lt;"; readonly '>': "&gt;"; readonly '"': "&quot;"; readonly "'": "&#39;"; }'.
    return s.replace(reUnescapedHtml, (c) => htmlEscapes[c]);
  }

  return s;
}
