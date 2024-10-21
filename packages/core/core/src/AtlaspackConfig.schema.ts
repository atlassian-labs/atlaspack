import type {PackageName} from '@atlaspack/types';
import type {SchemaEntity} from '@atlaspack/utils';

// Parcel validates plugin package names due to:
//
// * https://github.com/parcel-bundler/parcel/issues/3397#issuecomment-521353931
//
// Ultimately:
//
// * We do not care about package names
// * Validation makes interop between parcel/atlaspack confusing.
//
export function validatePackageName(
  // eslint-disable-next-line no-unused-vars
  pkg: PackageName | null | undefined,
  // eslint-disable-next-line no-unused-vars
  pluginType: string,
  // eslint-disable-next-line no-unused-vars
  key: string,
) {}

const validatePluginName = (pluginType: string, key: string) => {
  return (val: string) => {
    // allow plugin spread...
    if (val === '...') return;

    try {
      validatePackageName(val, pluginType, key);
    } catch (e: any) {
      return e.message;
    }
  };
};

const validateExtends = (val: string): void => {
  // allow relative paths...
  if (val.startsWith('.')) return;

  try {
    validatePackageName(val, 'config', 'extends');
  } catch (e: any) {
    return e.message;
  }
};

const pipelineSchema = (pluginType: string, key: string): SchemaEntity => {
  return {
    type: 'array',
    items: {
      type: 'string',
      __validate: validatePluginName(pluginType, key),
    },
  };
};

const mapPipelineSchema = (pluginType: string, key: string): SchemaEntity => {
  return {
    type: 'object',
    properties: {},
    additionalProperties: pipelineSchema(pluginType, key),
  };
};

const mapStringSchema = (pluginType: string, key: string): SchemaEntity => {
  return {
    type: 'object',
    properties: {},
    additionalProperties: {
      type: 'string',
      __validate: validatePluginName(pluginType, key),
    },
  };
};

export default {
  type: 'object',
  properties: {
    $schema: {
      type: 'string',
    },
    extends: {
      oneOf: [
        {
          type: 'string',
          __validate: validateExtends,
        },
        {
          type: 'array',
          items: {
            type: 'string',
            __validate: validateExtends,
          },
        },
      ],
    },
    bundler: {
      type: 'string',
      __validate: (validatePluginName('bundler', 'bundler') as (arg1: string) => void),
    },
    resolvers: (pipelineSchema('resolver', 'resolvers') as SchemaEntity),
    transformers: (mapPipelineSchema(
      'transformer',
      'transformers',
    ) as SchemaEntity),
    validators: (mapPipelineSchema('validator', 'validators') as SchemaEntity),
    namers: (pipelineSchema('namer', 'namers') as SchemaEntity),
    packagers: (mapStringSchema('packager', 'packagers') as SchemaEntity),
    optimizers: (mapPipelineSchema('optimizer', 'optimizers') as SchemaEntity),
    compressors: (mapPipelineSchema('compressor', 'compressors') as SchemaEntity),
    reporters: (pipelineSchema('reporter', 'reporters') as SchemaEntity),
    runtimes: (pipelineSchema('runtime', 'runtimes') as SchemaEntity),
    filePath: {
      type: 'string',
    },
    resolveFrom: {
      type: 'string',
    },
  },
  additionalProperties: false,
};
