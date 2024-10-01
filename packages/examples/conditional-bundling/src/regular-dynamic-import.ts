export const DynamicExport = () => 'This is a DynamicExport';
export const DynamicExportWithCondition = () => {
  return importCond<
    typeof import('./async-feature-enabled'),
    typeof import('./async-feature-disabled')
  >(
    'feature.async.condition',
    './async-feature-enabled.ts',
    './async-feature-disabled.ts',
    // @ts-expect-error - TS2339 - Property 'Feature' does not exist on type 'ConditionalImport<typeof import("/home/ubuntu/parcel/packages/examples/conditional-bundling/src/async-feature-enabled"), typeof import("/home/ubuntu/parcel/packages/examples/conditional-bundling/src/async-feature-disabled")>'.
  ).Feature();
};
