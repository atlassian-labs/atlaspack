// @flow
import {Transformer} from '@atlaspack/plugin';
import path from 'path';
import {EOL} from 'os';
import SourceMap from '@parcel/source-map';
import sass from 'sass';
import {promisify} from 'util';

// E.g: ~library/file.sass
const NODE_MODULE_ALIAS_RE = /^~[^/\\]/;

export default (new Transformer({
  async loadConfig({config, options}) {
    let configFile = await config.getConfig(
      ['.sassrc', '.sassrc.json', '.sassrc.js', '.sassrc.cjs', '.sassrc.mjs'],
      {
        packageKey: 'sass',
      },
    );

    let configResult = configFile ? configFile.contents : {};

    // Some packages in the wild declare a field `sass` in the package.json that
    // is a relative path to the sass entrypoint. In those cases we simply
    // initialize the config to an empty object.
    if (typeof configResult === 'string') {
      configResult = {};
    }

    // Resolve relative paths from config file
    if (configFile && configResult.includePaths) {
      configResult.includePaths = configResult.includePaths.map((p) =>
        path.resolve(path.dirname(configFile.filePath), p),
      );
    }

    if (configResult.importer === undefined) {
      configResult.importer = [];
    } else if (!Array.isArray(configResult.importer)) {
      configResult.importer = [configResult.importer];
    }

    // Always emit sourcemap
    configResult.sourceMap = true;
    // sources are created relative to the directory of outFile
    configResult.outFile = path.join(options.projectRoot, 'style.css.map');
    configResult.omitSourceMapUrl = true;
    configResult.sourceMapContents = false;

    return configResult;
  },

  async transform({asset, options, config, resolve}) {
    let rawConfig = config ?? {};
    let sassRender = promisify(sass.render.bind(sass));
    let css;
    try {
      let code = await asset.getCode();
      let result = await sassRender({
        ...rawConfig,
        file: asset.filePath,
        // $FlowFixMe
        data: rawConfig.data ? rawConfig.data + EOL + code : code,
        importer: [
          // $FlowFixMe
          ...rawConfig.importer,
          resolvePathImporter({
            asset,
            resolve,
            // $FlowFixMe
            includePaths: rawConfig.includePaths,
            options,
          }),
        ],
        indentedSyntax:
          typeof rawConfig.indentedSyntax === 'boolean'
            ? rawConfig.indentedSyntax
            : asset.type === 'sass',
      });

      css = result.css;
      for (let included of result.stats.includedFiles) {
        if (included !== asset.filePath) {
          asset.invalidateOnFileChange(included);
        }
      }

      if (result.map != null) {
        let map = new SourceMap(options.projectRoot);
        map.addVLQMap(JSON.parse(result.map));
        asset.setMap(map);
      }
    } catch (err) {
      // Adapt the Error object for the reporter.
      err.fileName = err.file;
      err.loc = {
        line: err.line,
        column: err.column,
      };

      throw err;
    }

    asset.type = 'css';
    asset.setCode(css);
    return [asset];
  },
}): Transformer<mixed>);

function resolvePathImporter({asset, resolve, includePaths, options}) {
  // This is a reimplementation of the Sass resolution algorithm that uses Atlaspack's
  // FS and tracks all tried files so they are watched for creation.
  async function resolvePath(
    url,
    prev,
  ): Promise<{filePath: string, contents: string, ...} | void> {
    /*
      Imports are resolved by trying, in order:
        * Loading a file relative to the file in which the `@import` appeared.
        * Each custom importer.
        * Loading a file relative to the current working directory (This rule doesn't really make sense for Atlaspack).
        * Each load path in `includePaths`
        * Each load path specified in the `SASS_PATH` environment variable, which should be semicolon-separated on Windows and colon-separated elsewhere.

      See: https://sass-lang.com/documentation/js-api#importer
      See also: https://github.com/sass/dart-sass/blob/006e6aa62f2417b5267ad5cdb5ba050226fab511/lib/src/importer/node/implementation.dart
    */

    let paths = [path.dirname(prev)];
    if (includePaths) {
      paths.push(...includePaths);
    }

    asset.invalidateOnEnvChange('SASS_PATH');
    if (options.env.SASS_PATH) {
      paths.push(
        ...options.env.SASS_PATH.split(
          process.platform === 'win32' ? ';' : ':',
        ).map((p) => path.resolve(options.projectRoot, p)),
      );
    }

    const urls = [url];
    const urlFileName = path.basename(url);
    if (urlFileName[0] !== '_') {
      urls.push(path.join(path.dirname(url), `_${urlFileName}`));
    }

    if (url[0] !== '~') {
      for (let p of paths) {
        for (let u of urls) {
          const filePath = path.resolve(p, u);
          try {
            const contents = await asset.fs.readFile(filePath, 'utf8');
            return {
              filePath,
              contents,
            };
          } catch (err) {
            asset.invalidateOnFileCreate({filePath});
          }
        }
      }
    }

    // If none of the default sass rules apply, try Atlaspack's resolver.
    for (let u of urls) {
      if (NODE_MODULE_ALIAS_RE.test(u)) {
        u = u.slice(1);
      }
      try {
        const filePath = await resolve(prev, u, {
          packageConditions: ['sass', 'style'],
        });
        if (filePath) {
          const contents = await asset.fs.readFile(filePath, 'utf8');
          return {filePath, contents};
        }
      } catch (err) {
        continue;
      }
    }
  }

  return function (rawUrl, prev, done) {
    const url = rawUrl.replace(/^file:\/\//, '');
    resolvePath(url, prev)
      .then((resolved) => {
        if (resolved) {
          done({
            file: resolved.filePath,
            contents: resolved.contents,
          });
        } else {
          done();
        }
      })
      .catch(done);
  };
}
