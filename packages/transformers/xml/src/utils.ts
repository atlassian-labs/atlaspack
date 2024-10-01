import type {MutableAsset} from '@atlaspack/types';

export function urlHandler(element: Element, asset: MutableAsset) {
  // @ts-expect-error - TS2531 - Object is possibly 'null'.
  element.textContent = asset.addURLDependency(element.textContent.trim(), {
    needsStableName: true,
  });
}
