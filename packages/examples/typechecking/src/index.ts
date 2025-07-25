type Params = {
  hello: string;
};

export default function test(params: Params) {
  // @ts-expect-error TS2339
  return params.world;
}
