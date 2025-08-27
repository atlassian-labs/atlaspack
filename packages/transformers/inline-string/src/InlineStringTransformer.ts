import {encodeJSONKeyComponent} from '@atlaspack/diagnostic';
import {Transformer} from '@atlaspack/plugin';
import {validateSchema, SchemaEntity} from '@atlaspack/utils';

type InlineStringTransformerConfig = {
  inlineThreshold: number | undefined;
};

const CONFIG_SCHEMA: SchemaEntity = {
  type: 'object',
  properties: {
    inlineThreshold: {
      type: 'number',
    },
  },
  additionalProperties: false,
};

export default new Transformer({
  async loadConfig({options, config}): Promise<InlineStringTransformerConfig> {
    let packageKey = '@atlaspack/transformer-inline-string';
    let conf = await config.getConfigFrom<InlineStringTransformerConfig>(
      `${process.cwd()}/index`,
      [],
      {
        packageKey,
      },
    );

    if (conf?.contents) {
      validateSchema.diagnostic(
        CONFIG_SCHEMA,
        {
          data: conf?.contents,
          source: await options.inputFS.readFile(conf.filePath, 'utf8'),
          filePath: conf.filePath,
          prependKey: `/${encodeJSONKeyComponent(packageKey)}`,
        },
        packageKey,
        `Invalid config for ${packageKey}`,
      );
    }
    return {inlineThreshold: conf?.contents?.inlineThreshold};
  },
  async transform({asset, config}) {
    let size = await asset.getBuffer().then((b) => b.length);
    let isBelowSizeThreshold =
      config.inlineThreshold == null
        ? true
        : (await asset.getBuffer()).length < config.inlineThreshold;

    if (isBelowSizeThreshold) {
      asset.bundleBehavior = 'inline';
      asset.meta.inlineType = 'string';
    }

    return [asset];
  },
}) as Transformer<unknown>;
