// @flow
// https://developer.mozilla.org/en-US/docs/Web/Manifest
import type {SchemaEntity} from '@atlaspack/utils';

import invariant from 'assert';
import {parse} from '@mischnic/json-sourcemap';
import {getJSONSourceLocation} from '@atlaspack/diagnostic';
import {Transformer} from '@atlaspack/plugin';
import {validateSchema} from '@atlaspack/utils';

const RESOURCES_SCHEMA = {
  type: 'array',
  items: {
    type: 'object',
    properties: {
      src: {
        type: 'string',
        __validate: s => {
          if (s.length === 0) {
            return 'Must not be empty';
          }
        },
      },
    },
    required: ['src'],
  },
};
const MANIFEST_SCHEMA: SchemaEntity = {
  type: 'object',
  properties: {
    icons: RESOURCES_SCHEMA,
    screenshots: RESOURCES_SCHEMA,
    shortcuts: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          icons: RESOURCES_SCHEMA,
        },
      },
    },
    file_handlers: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          icons: RESOURCES_SCHEMA,
        },
      },
    },
  },
};

export default (new Transformer({
  async transform({asset}) {
    const source = await asset.getCode();
    const {data, pointers} = parse(source);

    validateSchema.diagnostic(
      MANIFEST_SCHEMA,
      {source, map: {data, pointers}, filePath: asset.filePath},
      '@atlaspack/transformer-webmanifest',
      'Invalid webmanifest',
    );

    function addResourceListToAsset(list, parent) {
      if (list) {
        invariant(Array.isArray(list));
        for (let i = 0; i < list.length; i++) {
          const res = list[i];
          res.src = asset.addURLDependency(res.src, {
            loc: {
              filePath: asset.filePath,
              ...getJSONSourceLocation(
                pointers[`/${parent}/${i}/src`],
                'value',
              ),
            },
          });
        }
      }
    }

    for (const key of ['icons', 'screenshots']) {
      const list = data[key];
      addResourceListToAsset(list, key);
    }

    for (const key of ['shortcuts', 'file_handlers']) {
      const list = data[key];
      if (list) {
        invariant(Array.isArray(list));
        for (let i = 0; i < list.length; i++) {
          const iconList = list[i].icons;
          addResourceListToAsset(iconList, `${key}/${i}/icons`);
        }
      }
    }

    asset.type = 'webmanifest';
    asset.setCode(JSON.stringify(data));
    return [asset];
  },
}): Transformer);
