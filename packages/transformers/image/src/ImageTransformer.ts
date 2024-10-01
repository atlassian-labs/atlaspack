import {validateConfig} from './validateConfig';
import {Transformer} from '@atlaspack/plugin';
import nullthrows from 'nullthrows';
import WorkerFarm from '@atlaspack/workers';
// @ts-expect-error - TS1192 - Module '"/home/ubuntu/parcel/packages/transformers/image/src/loadSharp"' has no default export.
import loadSharp from './loadSharp';

// from https://github.com/lovell/sharp/blob/df7b8ba73808fc494be413e88cfb621b6279218c/lib/output.js#L6-L17
const FORMATS = new Map([
  ['jpeg', 'jpeg'],
  ['jpg', 'jpeg'],
  ['png', 'png'],
  ['webp', 'webp'],
  ['gif', 'gif'],
  ['tiff', 'tiff'],
  ['avif', 'avif'],
  ['heic', 'heif'],
  ['heif', 'heif'],
]);

let isSharpLoadedOnMainThread = false;

export default new Transformer({
  async loadConfig({config}) {
    let configFile: any = await config.getConfig(
      ['sharp.config.json'], // '.sharprc', '.sharprc.json'
      {packageKey: 'sharp'},
    );

    if (configFile?.contents) {
      validateConfig(configFile.contents, configFile.filePath);
      return configFile.contents;
    } else {
      return {};
    }
  },

  async transform({config, asset, options}) {
    asset.bundleBehavior = 'isolated';

    const originalFormat = FORMATS.get(asset.type);
    if (!originalFormat) {
      throw new Error(
        `The image transformer does not support ${asset.type} images.`,
      );
    }

    const width = asset.query.has('width')
      ? // @ts-expect-error - TS2345 - Argument of type 'string | null' is not assignable to parameter of type 'string'.
        parseInt(asset.query.get('width'), 10)
      : null;
    const height = asset.query.has('height')
      ? // @ts-expect-error - TS2345 - Argument of type 'string | null' is not assignable to parameter of type 'string'.
        parseInt(asset.query.get('height'), 10)
      : null;
    const quality = asset.query.has('quality')
      ? // @ts-expect-error - TS2345 - Argument of type 'string | null' is not assignable to parameter of type 'string'.
        parseInt(asset.query.get('quality'), 10)
      : // @ts-expect-error - TS2571 - Object is of type 'unknown'.
        config.quality;
    let targetFormat = asset.query.get('as')?.toLowerCase().trim();
    if (targetFormat && !FORMATS.has(targetFormat)) {
      throw new Error(
        `The image transformer does not support ${targetFormat} images.`,
      );
    }

    const format = nullthrows(FORMATS.get(targetFormat || originalFormat));
    // @ts-expect-error - TS2571 - Object is of type 'unknown'.
    const outputOptions = config[format];

    if (width || height || quality || targetFormat || outputOptions) {
      // Sharp must be required from the main thread as well to prevent errors when workers exit
      // See https://sharp.pixelplumbing.com/install#worker-threads and https://github.com/lovell/sharp/issues/2263
      // @ts-expect-error - TS2339 - Property 'isWorker' does not exist on type 'typeof WorkerFarm'.
      if (WorkerFarm.isWorker() && !isSharpLoadedOnMainThread) {
        // @ts-expect-error - TS2339 - Property 'getWorkerApi' does not exist on type 'typeof WorkerFarm'.
        let api = WorkerFarm.getWorkerApi();
        await api.callMaster({
          location: __dirname + '/loadSharp.js',
          args: [
            options.packageManager,
            asset.filePath,
            options.shouldAutoInstall,
          ],
        });

        isSharpLoadedOnMainThread = true;
      }

      let inputBuffer = await asset.getBuffer();
      let sharp = await loadSharp(
        options.packageManager,
        asset.filePath,
        options.shouldAutoInstall,
        true,
      );

      let imagePipeline = sharp(inputBuffer, {animated: true});

      imagePipeline.withMetadata();

      if (width || height) {
        imagePipeline.resize(width, height);
      }

      imagePipeline.rotate();

      const normalizedOutputOptions = outputOptions || {};
      if (format === 'jpeg') {
        normalizedOutputOptions.mozjpeg =
          normalizedOutputOptions.mozjpeg ?? true;
      }
      imagePipeline[format]({
        quality,
        ...normalizedOutputOptions,
      });

      asset.type = format;

      let buffer = await imagePipeline.toBuffer();
      asset.setBuffer(buffer);
    }

    return [asset];
  },
}) as Transformer;
