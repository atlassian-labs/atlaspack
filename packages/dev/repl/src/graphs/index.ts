import {wrap} from 'comlink';

let worker;

export default (() =>
  (worker =
    worker ??
    wrap(
      // $FlowFixMe
      new Worker(new URL('./worker.js', import /*:: ("") */.meta.url), {
        name: 'Atlaspack Graph Renderer',
        type: 'module',
      }),
    ).render)) as () => Promise<(dot: string) => Promise<string>>;
