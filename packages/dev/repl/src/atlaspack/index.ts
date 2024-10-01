import type {FS, REPLOptions} from '../utils';
import type {BundleOutput} from './AtlaspackWorker';

import {proxy, wrap, transfer} from 'comlink';

const worker = wrap(
  // $FlowFixMe
  // @ts-expect-error - TS1343 - The 'import.meta' meta-property is only allowed when the '--module' option is 'es2020', 'es2022', 'esnext', 'system', 'node12', or 'nodenext'.
  new Worker(new URL('./ParcelWorker.js', import /*:: ("") */.meta.url), {
    name: 'Atlaspack Worker Main',
    type: 'module',
  }),
);

// const worker = {
//   waitForFS: () => Promise.resolve(),
//   ready: Promise.resolve(),
//   bundle(assets, options, progress): Promise<BundleOutput> {
//     return Promise.resolve({
//       type: 'success',
//       bundles: assets.map(({name, content}) => ({
//         name,
//         content,
//         time: 0,
//         size: content.length,
//       })),
//       buildTime: 1,
//       graphs: options.renderGraphs
//         ? [
//             {
//               name: 'test',
//               content: `digraph graphname
// {
//     a -> b -> c;
//     b -> d;
// }`,
//             },
//           ]
//         : null,
//       sourcemaps: null,
//     });
//   },
//   watch(...args) {
//     return Promise.resolve({
//       unsubscribe: () => Promise.resolve(),
//       writeAssets: () => Promise.resolve(args),
//     });
//   },
//   setServiceWorker: v => v,
// };

export function workerReady(numWorkers?: number | null): Promise<void> {
  // @ts-expect-error - TS2339 - Property 'ready' does not exist on type 'Remote<unknown>'.
  return worker.ready(numWorkers);
}

export function waitForFS(): Promise<void> {
  // @ts-expect-error - TS2339 - Property 'waitForFS' does not exist on type 'Remote<unknown>'.
  return worker.waitForFS();
}

export function bundle(
  files: FS,
  options: REPLOptions,
  progress: (arg1: string) => void,
): Promise<BundleOutput> {
  // @ts-expect-error - TS2339 - Property 'bundle' does not exist on type 'Remote<unknown>'.
  return worker.bundle(files.toJSON(), options, proxy(progress));
}

export async function watch(
  files: FS,
  options: REPLOptions,
  onBuild: (arg1: BundleOutput) => void,
  progress: (arg1?: string | null | undefined) => void,
): Promise<{
  unsubscribe: () => Promise<unknown>;
  writeAssets: (arg1: FS) => Promise<unknown>;
}> {
  // @ts-expect-error - TS2339 - Property 'watch' does not exist on type 'Remote<unknown>'.
  let result = await worker.watch(
    files.toJSON(),
    options,
    proxy(onBuild),
    proxy(progress),
  );
  return {
    unsubscribe: result.unsubscribe,
    writeAssets: (f) => result.writeAssets(f.toJSON()),
  };
}

class MessageTarget {
  receive: any;
  post: any;
  constructor(receive: any, post: any) {
    this.receive = receive;
    this.post = post;
  }
  // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type.
  postMessage(...args) {
    this.post.postMessage(...args);
  }
  // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type.
  addEventListener(...args) {
    this.receive.addEventListener(...args);
  }
  // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type.
  removeEventListener(...args) {
    this.receive.removeEventListener(...args);
  }
  sendMsg(type: string, data: undefined, transfer: undefined) {
    let id = uuidv4();
    return new Promise((res: (result: Promise<never>) => void) => {
      let handler = (evt: any) => {
        if (evt.data.id === id) {
          this.removeEventListener('message', handler);
          res(evt.data.data);
        }
      };
      this.addEventListener('message', handler);
      this.postMessage({type, data, id}, transfer);
    });
  }
}

function uuidv4() {
  return (String(1e7) + -1e3 + -4e3 + -8e3 + -1e11).replace(
    /[018]/g,
    // $FlowFixMe
    // @ts-expect-error - TS2769 - No overload matches this call.
    (c: number) =>
      (
        c ^
        // $FlowFixMe
        (crypto.getRandomValues(new Uint8Array(1))[0] & (15 >> (c / 4)))
      ).toString(16),
  );
}

export let clientID: Promise<string> = Promise.resolve('no-sw');

if (navigator.serviceWorker) {
  clientID = (async () => {
    let {active: serviceWorker} = await navigator.serviceWorker.ready;

    let sw = new MessageTarget(navigator.serviceWorker, serviceWorker);

    let {port1, port2} = new MessageChannel();

    // sw <-> port1 <-> port2 <-> parcel worker thread
    // sw <-> main thread

    sw.addEventListener('message', (evt: MessageEvent) => {
      port2.postMessage(evt.data);
    });
    port2.addEventListener('message', (evt: MessageEvent) => {
      sw.postMessage(evt.data);
    });

    port2.start();
    // @ts-expect-error - TS2339 - Property 'setServiceWorker' does not exist on type 'Remote<unknown>'.
    await worker.setServiceWorker(transfer(port1, [port1]));

    // @ts-expect-error - TS2554 - Expected 3 arguments, but got 1.
    return sw.sendMsg('getID');
  })();
}
