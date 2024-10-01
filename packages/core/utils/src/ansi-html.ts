// @ts-expect-error - TS7016 - Could not find a declaration file for module 'ansi-html-community'. '/home/ubuntu/parcel/node_modules/ansi-html-community/index.js' implicitly has an 'any' type.
import ansiHTML from 'ansi-html-community';
import {escapeHTML} from './escape-html';

export function ansiHtml(ansi: string): string {
  return ansiHTML(escapeHTML(ansi));
}
