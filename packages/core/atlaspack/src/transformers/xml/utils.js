// @flow
import type {MutableAsset} from '../../types/index.js';

export function urlHandler(element: Element, asset: MutableAsset) {
  element.textContent = asset.addURLDependency(element.textContent.trim(), {
    needsStableName: true,
  });
}
