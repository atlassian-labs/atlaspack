import {wrap} from 'comlink';

// @ts-expect-error - TS7034 - Variable 'worker' implicitly has type 'any' in some locations where its type cannot be determined.
let worker;

export default (() =>
  (worker =
    // @ts-expect-error - TS7005 - Variable 'worker' implicitly has an 'any' type.
    worker ??
    wrap(
      // $FlowFixMe
      // @ts-expect-error - TS1343 - The 'import.meta' meta-property is only allowed when the '--module' option is 'es2020', 'es2022', 'esnext', 'system', 'node12', or 'nodenext'.
      new Worker(new URL('./worker.js', import /*:: ("") */.meta.url), {
        name: 'Atlaspack Graph Renderer',
        type: 'module',
      }),
      // @ts-expect-error - TS2339 - Property 'render' does not exist on type 'Remote<unknown>'.
    ).render)) as () => Promise<(dot: string) => Promise<string>>;
