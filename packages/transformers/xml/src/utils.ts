import type {MutableAsset} from '@atlaspack/types-internal';

export function urlHandler(element: Element, asset: MutableAsset) {
  // @ts-expect-error TS18047
  element.textContent = asset.addURLDependency(element.textContent.trim(), {
    needsStableName: true,
  });
}
