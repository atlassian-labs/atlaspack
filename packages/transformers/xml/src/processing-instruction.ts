import type {MutableAsset} from '@atlaspack/types';
import {DOMParser, XMLSerializer} from '@xmldom/xmldom';

// Flow doesn't define ProcessingInstruction by default.
type ProcessingInstruction = CharacterData;

module.exports = {
  'xml-stylesheet': (node: ProcessingInstruction, asset: MutableAsset) => {
    const pseudo = new DOMParser().parseFromString(`<ψ ${node.data} />`);

    // @ts-expect-error - TS2531 - Object is possibly 'null'. | TS2339 - Property 'getAttribute' does not exist on type 'ChildNode'.
    const input = pseudo.firstChild.getAttribute('href');
    const output = asset.addURLDependency(input, {priority: 'parallel'});
    // @ts-expect-error - TS2531 - Object is possibly 'null'. | TS2339 - Property 'setAttribute' does not exist on type 'ChildNode'.
    pseudo.firstChild.setAttribute('href', output);

    node.data = new XMLSerializer().serializeToString(pseudo).slice(2, -2);
  },
};
