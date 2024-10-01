import {Reporter} from '@atlaspack/plugin';
import HMRServer, {getHotAssetContents} from './HMRServer';

// @ts-expect-error - TS7034 - Variable 'hmrServer' implicitly has type 'any' in some locations where its type cannot be determined.
let hmrServer;
let hmrAssetSourceCleanup: (() => void) | undefined;

export default new Reporter({
  async report({event, options}) {
    let {hmrOptions} = options;
    switch (event.type) {
      case 'watchStart': {
        if (hmrOptions) {
          hmrServer = new HMRServer((data: HMRMessage) =>
            // $FlowFixMe
            // @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
            globalThis.ATLASPACK_SERVICE_WORKER('hmrUpdate', data),
          );
        }
        break;
      }
      case 'watchEnd':
        break;
      case 'buildStart':
        break;
      case 'buildSuccess':
        {
          let files: {
            [key: string]: string;
          } = {};
          for (let f of await options.outputFS.readdir('/app/dist')) {
            files[f] = await options.outputFS.readFile(
              '/app/dist/' + f,
              'utf8',
            );
          }
          // @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
          await globalThis.ATLASPACK_SERVICE_WORKER('setFS', files);

          hmrAssetSourceCleanup?.();
          // @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
          hmrAssetSourceCleanup = globalThis.ATLASPACK_SERVICE_WORKER_REGISTER(
            'hmrAssetSource',
            // @ts-expect-error - TS7006 - Parameter 'id' implicitly has an 'any' type.
            async (id) => {
              let bundleGraph = event.bundleGraph;
              let asset = bundleGraph.getAssetById(id);
              return [
                asset.type,
                await getHotAssetContents(bundleGraph, asset),
              ];
            },
          );

          // @ts-expect-error - TS7005 - Variable 'hmrServer' implicitly has an 'any' type.
          if (hmrServer) {
            // @ts-expect-error - TS7005 - Variable 'hmrServer' implicitly has an 'any' type.
            await hmrServer?.emitUpdate(event);
          }
        }
        break;
      // We show this in the "frontend" as opposed to the iframe
      // case 'buildFailure':
      //   await hmrServer?.emitError(options, event.diagnostics);
      //   break;
    }
  },
}) as Reporter;
