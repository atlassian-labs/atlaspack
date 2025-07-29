import {wrap} from 'comlink';

// @ts-expect-error TS7034
let worker;

export default (() =>
  (worker =
    // @ts-expect-error TS7005
    worker ??
    wrap(
      // @ts-expect-error TS1470
      new Worker(new URL('./worker.js', import /*:: ("") */.meta.url), {
        name: 'Atlaspack Graph Renderer',
        type: 'module',
      }),
      // @ts-expect-error TS2339
    ).render)) as () => Promise<(dot: string) => Promise<string>>;
