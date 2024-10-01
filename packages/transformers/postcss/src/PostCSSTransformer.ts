import type {
  FilePath,
  Asset,
  MutableAsset,
  PluginOptions,
} from '@atlaspack/types';

import {hashString} from '@atlaspack/rust';
import {glob} from '@atlaspack/utils';
import {Transformer} from '@atlaspack/plugin';
import nullthrows from 'nullthrows';
import path from 'path';
import semver from 'semver';
import valueParser from 'postcss-value-parser';
import * as Postcss from 'postcss';

import {load} from './loadConfig';
import {POSTCSS_RANGE} from './constants';
import {md, generateJSONCodeHighlights} from '@atlaspack/diagnostic';

const COMPOSES_RE = /composes:.+from\s*("|').*("|')\s*;?/;
const FROM_IMPORT_RE = /.+from\s*(?:"|')(.*)(?:"|')\s*;?/;
const LEGACY_MODULE_RE = /@value|:export|(:global|:local|:import)(?!\s*\()/i;
const MODULE_BY_NAME_RE = /\.module\./;

export default new Transformer({
  loadConfig({config, options, logger}) {
    return load({config, options, logger});
  },

  canReuseAST({ast}) {
    return (
      ast.type === 'postcss' && semver.satisfies(ast.version, POSTCSS_RANGE)
    );
  },

  async parse({asset, config, options}) {
    let isLegacy = await isLegacyCssModule(asset);
    if (!config && !isLegacy) {
      return;
    }

    const postcss = await loadPostcss(options, asset.filePath);

    return {
      type: 'postcss',
      version: '8.2.1',
      program: postcss
        .parse(await asset.getCode(), {
          from: asset.filePath,
        })
        .toJSON(),
    };
  },

  async transform({asset, config, options, resolve, logger}) {
    asset.type = 'css';
    let isLegacy = await isLegacyCssModule(asset);
    if (isLegacy && !config) {
      config = {
        raw: {},
        filePath: '',
        hydrated: {
          plugins: [],
          from: asset.filePath,
          to: asset.filePath,
          modules: {},
        },
      };

      // TODO: warning?
    }

    if (!config) {
      return [asset];
    }

    // @ts-expect-error - TS2709 - Cannot use namespace 'Postcss' as a type.
    const postcss: Postcss = await loadPostcss(options, asset.filePath);
    let ast = nullthrows(await asset.getAST());
    let program = postcss.fromJSON(ast.program);

    // @ts-expect-error - TS2571 - Object is of type 'unknown'.
    let plugins = [...config.hydrated.plugins];
    let cssModules:
      | {
          [key: string]: string;
        }
      | null
      | undefined = null;
    // @ts-expect-error - TS2571 - Object is of type 'unknown'.
    if (config.hydrated.modules) {
      asset.meta.cssModulesCompiled = 'postcss';

      let code = asset.isASTDirty() ? null : await asset.getCode();
      if (
        // @ts-expect-error - TS2571 - Object is of type 'unknown'.
        Object.keys(config.hydrated.modules).length === 0 &&
        code &&
        !isLegacy &&
        !LEGACY_MODULE_RE.test(code)
      ) {
        // @ts-expect-error - TS2571 - Object is of type 'unknown'.
        let filename = path.basename(config.filePath);
        let message;
        let configKey;
        let hint;
        // @ts-expect-error - TS2571 - Object is of type 'unknown'.
        if (config.raw.modules) {
          // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
          message = md`The "modules" option in __${filename}__ can be replaced with configuration for @atlaspack/transformer-css to improve build performance.`;
          configKey = '/modules';
          // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
          hint = md`Remove the "modules" option from __${filename}__`;
        } else {
          // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
          message = md`The "postcss-modules" plugin in __${filename}__ can be replaced with configuration for @atlaspack/transformer-css to improve build performance.`;
          configKey = '/plugins/postcss-modules';
          // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
          hint = md`Remove the "postcss-modules" plugin from __${filename}__`;
        }
        if (filename === 'package.json') {
          configKey = `/postcss${configKey}`;
        }

        let hints = [
          'Enable the "cssModules" option for "@atlaspack/transformer-css" in your package.json',
        ];
        if (plugins.length === 0) {
          // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
          message += md` Since there are no other plugins, __${filename}__ can be deleted safely.`;
          // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
          hints.push(md`Delete __${filename}__`);
        } else {
          hints.push(hint);
        }

        let codeFrames;
        if (path.extname(filename) !== '.js') {
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          let contents = await asset.fs.readFile(config.filePath, 'utf8');
          codeFrames = [
            {
              language: 'json',
              // @ts-expect-error - TS2571 - Object is of type 'unknown'.
              filePath: config.filePath,
              code: contents,
              codeHighlights: generateJSONCodeHighlights(contents, [
                {
                  key: configKey,
                  type: 'key',
                },
              ]),
            },
          ];
        } else {
          codeFrames = [
            {
              // @ts-expect-error - TS2571 - Object is of type 'unknown'.
              filePath: config.filePath,
              codeHighlights: [
                {
                  start: {line: 1, column: 1},
                  end: {line: 1, column: 1},
                },
              ],
            },
          ];
        }

        logger.warn({
          message,
          hints,
          documentationURL:
            'https://parceljs.org/languages/css/#enabling-css-modules-globally',
          codeFrames,
        });
      }

      // TODO: should this be resolved from the project root?
      let postcssModules = await options.packageManager.require(
        'postcss-modules',
        asset.filePath,
        {
          range: '^4.3.0',
          saveDev: true,
          shouldAutoInstall: options.shouldAutoInstall,
        },
      );

      plugins.push(
        postcssModules({
          // @ts-expect-error - TS7006 - Parameter 'filename' implicitly has an 'any' type. | TS7006 - Parameter 'json' implicitly has an 'any' type.
          getJSON: (filename, json) => (cssModules = json),
          Loader: await createLoader(asset, resolve, options),
          // @ts-expect-error - TS7006 - Parameter 'name' implicitly has an 'any' type. | TS7006 - Parameter 'filename' implicitly has an 'any' type.
          generateScopedName: (name, filename) =>
            `${name}_${hashString(
              path.relative(options.projectRoot, filename),
            ).substr(0, 6)}`,
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          ...config.hydrated.modules,
        }),
      );

      if (code == null || COMPOSES_RE.test(code)) {
        // @ts-expect-error - TS7006 - Parameter 'decl' implicitly has an 'any' type.
        program.walkDecls((decl) => {
          let [, importPath] = FROM_IMPORT_RE.exec(decl.value) || [];
          if (decl.prop === 'composes' && importPath != null) {
            let parsed = valueParser(decl.value);

            parsed.walk((node) => {
              if (node.type === 'string') {
                asset.addDependency({
                  specifier: importPath,
                  specifierType: 'url',
                  loc: {
                    filePath: asset.filePath,
                    start: decl.source.start,
                    end: {
                      line: decl.source.start.line,
                      column: decl.source.start.column + importPath.length,
                    },
                  },
                });
              }
            });
          }
        });
      }
    }

    let {messages, root} = await postcss(plugins).process(
      program,
      // @ts-expect-error - TS2571 - Object is of type 'unknown'.
      config.hydrated,
    );
    asset.setAST({
      type: 'postcss',
      version: '8.2.1',
      program: root.toJSON(),
    });
    for (let msg of messages) {
      if (msg.type === 'dependency') {
        asset.invalidateOnFileChange(msg.file);
      } else if (msg.type === 'dir-dependency') {
        let pattern = `${msg.dir}/${msg.glob ?? '**/*'}`;
        let files = await glob(pattern, asset.fs, {onlyFiles: true});
        for (let file of files) {
          asset.invalidateOnFileChange(path.normalize(file));
        }
        asset.invalidateOnFileCreate({glob: pattern});
      }
    }

    let assets = [asset];
    if (cssModules) {
      let cssModulesList = Object.entries(cssModules) as Array<
        [string, string]
      >;
      let deps = asset
        .getDependencies()
        .filter((dep) => dep.priority === 'sync');
      let code: string;
      if (deps.length > 0) {
        code = `
          module.exports = Object.assign({}, ${deps
            .map((dep) => `require(${JSON.stringify(dep.specifier)})`)
            .join(', ')}, ${JSON.stringify(cssModules, null, 2)});
        `;
      } else {
        code = cssModulesList
          .map(
            // This syntax enables shaking the invidual statements, so that unused classes don't even exist in JS.
            ([className, classNameHashed]: [any, any]) =>
              `module.exports[${JSON.stringify(className)}] = ${JSON.stringify(
                classNameHashed,
              )};`,
          )
          .join('\n');
      }

      asset.symbols.ensure();
      for (let [k, v] of cssModulesList) {
        // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
        asset.symbols.set(k, v);
      }
      // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
      asset.symbols.set('default', 'default');

      assets.push({
        type: 'js',
        // @ts-expect-error - TS2345 - Argument of type '{ type: string; content: string; }' is not assignable to parameter of type 'MutableAsset'.
        content: code,
      });
    }
    return assets;
  },

  async generate({asset, ast, options}) {
    // @ts-expect-error - TS2709 - Cannot use namespace 'Postcss' as a type.
    const postcss: Postcss = await loadPostcss(options, asset.filePath);

    let code = '';
    // @ts-expect-error - TS7006 - Parameter 'c' implicitly has an 'any' type.
    postcss.stringify(postcss.fromJSON(ast.program), (c) => {
      code += c;
    });

    return {
      content: code,
    };
  },
}) as Transformer;

async function createLoader(
  asset: MutableAsset,
  resolve: (from: FilePath, to: string) => Promise<FilePath>,
  options: PluginOptions,
) {
  let {default: FileSystemLoader} = await options.packageManager.require(
    'postcss-modules/build/css-loader-core/loader',
    asset.filePath,
  );
  return class AtlaspackFileSystemLoader extends FileSystemLoader {
    // @ts-expect-error - TS7023 - 'fetch' implicitly has return type 'any' because it does not have a return type annotation and is referenced directly or indirectly in one of its return expressions.
    async fetch(composesPath: any, relativeTo: any) {
      let importPath = composesPath.replace(/^["']|["']$/g, '');
      let resolved = await resolve(relativeTo, importPath);
      let rootRelativePath = path.resolve(path.dirname(relativeTo), resolved);
      let root = path.resolve('/');
      // fixes an issue on windows which is part of the css-modules-loader-core
      // see https://github.com/css-modules/css-modules-loader-core/issues/230
      if (rootRelativePath.startsWith(root)) {
        rootRelativePath = rootRelativePath.substr(root.length);
      }

      let source = await asset.fs.readFile(resolved, 'utf-8');
      // @ts-expect-error - TS7022 - 'exportTokens' implicitly has type 'any' because it does not have a type annotation and is referenced directly or indirectly in its own initializer.
      let {exportTokens} = await this.core.load(
        source,
        rootRelativePath,
        undefined,
        // $FlowFixMe[method-unbinding]
        this.fetch.bind(this),
      );
      return exportTokens;
    }

    get finalSource() {
      return '';
    }
  };
}

// @ts-expect-error - TS2709 - Cannot use namespace 'Postcss' as a type.
function loadPostcss(options: PluginOptions, from: FilePath): Promise<Postcss> {
  return options.packageManager.require('postcss', from, {
    range: POSTCSS_RANGE,
    saveDev: true,
    shouldAutoInstall: options.shouldAutoInstall,
  });
}

async function isLegacyCssModule(asset: Asset | MutableAsset) {
  if (!MODULE_BY_NAME_RE.test(asset.filePath)) {
    return false;
  }

  let code = await asset.getCode();
  return LEGACY_MODULE_RE.test(code);
}
