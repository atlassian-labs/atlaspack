import {Transformer} from '@atlaspack/plugin';
import {DOMParser, XMLSerializer} from '@xmldom/xmldom';
import * as atom from './atom';
import * as processingInstruction from './processing-instruction';
import * as rss from './rss';

const HANDLERS = {
  'http://www.w3.org/2005/Atom': atom,
} as const;

const NON_NAMESPACED_HANDLERS = {
  rss,
} as const;

export default new Transformer({
  async transform({asset}) {
    let code = await asset.getCode();
    let parser = new DOMParser();
    let dom = parser.parseFromString(code, 'application/xml');

    let parts: Array<any> = [];
    let nonNamespacedHandlers = !dom.documentElement.namespaceURI
      ? NON_NAMESPACED_HANDLERS[dom.documentElement.nodeName] || {}
      : {};

    walk(dom, (node) => {
      let handler =
        node.nodeType === node.ELEMENT_NODE
          ? node.namespaceURI
            ? HANDLERS[node.namespaceURI]?.[node.localName]
            : nonNamespacedHandlers[node.nodeName]
          : node.nodeType === node.PROCESSING_INSTRUCTION_NODE
          ? processingInstruction[node.target]
          : undefined;

      if (handler) {
        handler(node, asset, parts);
      }
    });

    code = new XMLSerializer().serializeToString(dom);
    asset.setCode(code);

    return [asset, ...parts];
  },
}) as Transformer;

function walk(element: any, visit: (node?: any) => void) {
  visit(element);

  element = element.firstChild;
  while (element) {
    walk(element, visit);
    element = element.nextSibling;
  }
}
