type Params = {
  hello: string;
};

export default function test(params: Params) {
  // @ts-expect-error - TS2339 - Property 'world' does not exist on type 'Params'.
  return params.world;
}
