import assert from 'assert';
import {extname, join} from 'path';

import {
  bundle,
  describe,
  distDir,
  inputFS,
  it,
  outputFS,
  removeDistDirectory,
} from '@atlaspack/test-utils';
import exifReader from 'exif-reader';
import sharp from 'sharp';

describe('images', function () {
  this.timeout(10000);

  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('can be resized with a query string', async () => {
    await bundle(join(__dirname, '../data/integration/image/resized.js'));

    let filenames = await outputFS.readdir(distDir);
    let exts = filenames.map(f => extname(f)).filter(ext => ext !== '.map');

    assert.deepStrictEqual(exts.sort(), ['.jpeg', '.js']);

    let filename = filenames.find(
      f => f.endsWith('.jpeg') || f.endsWith('.jpg'),
    );

    let buffer = await outputFS.readFile(join(distDir, filename));
    let image = await sharp(buffer).metadata();

    assert.equal(image.width, 600);
    assert.equal(image.height, 400);
  });

  describe('can be resized and reformatted with a query string', () => {
    function testCase(ext) {
      return async () => {
        await bundle(
          join(__dirname, `/integration/image-multiple-queries/index.${ext}`),
        );

        let filenames = await outputFS.readdir(distDir);
        let exts = filenames.map(f => extname(f)).filter(ext => ext !== '.map');

        assert.deepStrictEqual(
          exts.sort(),
          [`.${ext}`, '.jpeg', '.jpeg', '.webp'].sort(),
        );
      };
    }

    it('from javascript', testCase('js'));
    it.v2('from html', testCase('html'));
    it.v2('from css', testCase('css'));
  });

  it.v2(
    'can be resized and reformatted from html with a query string',
    async () => {},
  );

  describe('can change image format with a query string', () => {
    function testCase(ext) {
      return async () => {
        await bundle(join(__dirname, `/integration/image/reformat.${ext}`));

        let filenames = await outputFS.readdir(distDir);
        let exts = filenames.map(f => extname(f)).filter(ext => ext !== '.map');

        assert.deepStrictEqual(exts.sort(), [`.${ext}`, '.webp'].sort());
      };
    }

    it('from javascript', testCase('js'));
    it.v2('from html', testCase('html'));
    it.v2('from css', testCase('css'));

    it.v2('all formats', async () => {
      let b = await bundle(
        join(__dirname, `/integration/image/reformat-all.html`),
      );

      let exts = new Set(b.getBundles().map(({type}) => type));

      assert.deepStrictEqual(
        exts,
        new Set(['html', 'webp', 'avif', 'jpeg', 'png', 'tiff']),
      );
    });
  });

  it('are optimised as lossless jpg', async () => {
    let img = join(__dirname, '../data/integration/image/image.jpg');
    let b = await bundle(img, {
      defaultTargetOptions: {
        shouldOptimize: true,
      },
    });

    let {filePath} = b.getBundles().find(b => ['jpg', 'jpeg'].includes(b.type));

    let input = await inputFS.readFile(img);
    let inputRaw = await sharp(input).toFormat('raw').toBuffer();

    let output = await outputFS.readFile(filePath);
    let outputRaw = await sharp(output).toFormat('raw').toBuffer();

    assert(outputRaw.equals(inputRaw));
    assert(output.length < input.length);
  });

  it('are optimised as lossless progressive jpgs', async function () {
    let img = join(__dirname, '../data/integration/image/banana.jpg');
    let b = await bundle(img, {
      defaultTargetOptions: {
        shouldOptimize: true,
      },
      logLevel: 'verbose',
    });

    let {filePath} = b.getBundles().find(b => ['jpg', 'jpeg'].includes(b.type));

    // let input = await inputFS.readFile(img);
    // let inputRaw = await sharp(input)
    //   .toFormat('raw')
    //   .toBuffer();

    // Check validity of image
    let output = await outputFS.readFile(filePath);
    await sharp(output).toFormat('raw').toBuffer();

    // assert(outputRaw.equals(inputRaw));
    // assert(output.length < input.length);
  });

  it('are optimised as lossless pngs', async function () {
    let img = join(__dirname, '../data/integration/image/clock.png');
    let b = await bundle(img, {
      defaultTargetOptions: {
        shouldOptimize: true,
      },
    });

    let {filePath} = b.getBundles().find(b => b.type === 'png');

    let input = await inputFS.readFile(img);
    let inputRaw = await sharp(input).toFormat('raw').toBuffer();

    let output = await outputFS.readFile(filePath);
    let outputRaw = await sharp(output).toFormat('raw').toBuffer();

    assert(outputRaw.equals(inputRaw));
    assert(output.length < input.length);
  });

  it.v2('retain EXIF data when resized with a query string', async () => {
    let b = await bundle(join(__dirname, '../data/integration/image-exif/resized.js'));

    let {filePath} = b.getBundles().find(b => ['jpg', 'jpeg'].includes(b.type));

    let buffer = await outputFS.readFile(filePath);
    let image = await sharp(buffer).metadata();

    let {exif} = exifReader(image.exif);

    assert.strictEqual(
      exif.UserComment.toString(),
      'ASCII\u0000\u0000\u0000This is a comment',
    );
  });

  it('removes EXIF data when optimizing', async () => {
    let b = await bundle(
      join(__dirname, '../data/integration/image-exif/resized.js'),
      {
        defaultTargetOptions: {
          shouldOptimize: true,
        },
      },
    );

    let {filePath} = b.getBundles().find(b => ['jpg', 'jpeg'].includes(b.type));

    let buffer = await outputFS.readFile(filePath);
    let image = await sharp(buffer).metadata();

    assert.strictEqual(image.exif, undefined);
  });

  it.v2('uses the EXIF orientation tag when resizing', async () => {
    let b = await bundle(join(__dirname, '../data/integration/image-exif/resized.js'));

    let {filePath} = b.getBundles().find(b => ['jpg', 'jpeg'].includes(b.type));

    let buffer = await outputFS.readFile(filePath);
    let image = await sharp(buffer).metadata();

    assert.strictEqual(image.orientation, 1);
    assert.strictEqual(image.width, 240);
    assert.strictEqual(image.height, 320);
  });

  it.v2('support sharp config file for jpegs', async function () {
    let b = await bundle(
      join(__dirname, '../data/integration/image-config/image.jpg'),
      {
        defaultTargetOptions: {
          shouldOptimize: false,
        },
      },
    );

    let {filePath} = b.getBundles().find(b => b.type === 'jpeg');

    let buffer = await outputFS.readFile(filePath);
    let image = await sharp(buffer).metadata();
    let originalSize = 549196;

    assert.strictEqual(image.width, 1920);
    assert.strictEqual(image.chromaSubsampling, '4:4:4');
    assert(image.size < originalSize);
  });

  it.v2('support sharp config files for pngs', async function () {
    let b = await bundle(
      join(__dirname, '../data/integration/image-config/clock.png'),
      {
        defaultTargetOptions: {
          shouldOptimize: false,
        },
      },
    );

    let {filePath} = b.getBundles().find(b => b.type === 'png');

    let buffer = await outputFS.readFile(filePath);
    let image = await sharp(buffer).metadata();
    let originalSize = 84435;

    assert.strictEqual(image.width, 200);
    assert.strictEqual(image.paletteBitDepth, 8);
    assert(image.size < originalSize);
  });
});
