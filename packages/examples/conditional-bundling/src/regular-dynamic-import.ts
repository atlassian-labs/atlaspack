export const DynamicExport = () => 'This is a DynamicExport';
export const DynamicExportWithCondition = () => {
  // @ts-expect-error TS2304
  return importCond<
    typeof import('./async-feature-enabled'),
    typeof import('./async-feature-disabled')
  >(
    'feature.async.condition',
    './async-feature-enabled.ts',
    './async-feature-disabled.ts',
  ).Feature();
};
