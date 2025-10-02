import assert from 'assert';
import {join, basename} from 'path';

import {sort} from '@compiled/css';
import {hash} from '@compiled/utils';
import {Optimizer} from '@atlaspack/plugin';
import posthtml from 'posthtml';
import {insertAt} from 'posthtml-insert-at';

export interface OptimizerOpts {
  /**
   * Indicates whether CSS content is inlined in HTML or served as a external .css file.
   * Defaults to `false`.
   */
  inlineCss: boolean;

  /**
   * Whether to sort at-rules, including media queries.
   * Defaults to `true`.
   */
  sortAtRules: boolean;

  /**
   * Whether to sort shorthand and longhand properties,
   * eg. `margin` before `margin-top` for enforced determinism.
   * Defaults to `true`.
   */
  sortShorthand?: boolean;
}

const configFiles = [
  '.compiledcssrc',
  '.compiledcssrc.json',
  'compiledcss.js',
  'compiledcss.config.js',
];

export default new Optimizer<OptimizerOpts, unknown>({
  async loadConfig({config, options}) {
    const conf = await config.getConfigFrom(
      join(options.projectRoot, 'index'),
      configFiles,
      {
        packageKey: '@atlaspack/optimizer-compiled-css-in-js',
      },
    );

    const contents = {
      inlineCss: false,
      sortAtRules: true,
    };

    if (conf) {
      if (conf.filePath.endsWith('.js')) {
        config.invalidateOnStartup();
      }

      Object.assign(contents, conf.contents);
    }

    return contents;
  },

  async optimize({
    contents,
    map,
    bundle,
    bundleGraph,
    options,
    config,
    logger,
  }) {
    const {outputFS} = options;

    const styleRules = new Set<string>();

    console.log('optimize styleRules', bundle.displayName);

    // Traverse the descendants of HTML bundle
    // Extract the stylesRules from assets
    bundleGraph.traverseBundles((childBundle) => {
      childBundle.traverseAssets((asset) => {
        const rules = asset.getCompiledCssStyles();
        if (!rules) {
          return;
        }

        for (const rule of rules) {
          styleRules.add(rule);
        }
      });
    }, bundle);

    logger.info({
      message: 'optimize styleRules ' + Array.from(styleRules).join('\n'),
    });

    if (styleRules.size === 0) return {contents, map};

    const sortConfig = {
      sortAtRulesEnabled: config.sortAtRules,
      sortShorthandEnabled: config.sortShorthand,
    };
    const stylesheet = sort(Array.from(styleRules).join(''), sortConfig);

    let newContents = '';

    if (config.inlineCss) {
      newContents = (
        await posthtml()
          .use(
            insertAt({
              selector: 'head',
              append: '<style>' + stylesheet + '</style>',
              behavior: 'inside',
            }),
          )
          .process(contents.toString())
      ).html;
    } else {
      const {distDir} = bundle.target;

      if (!outputFS.existsSync(distDir)) {
        await outputFS.mkdirp(distDir);
      }

      const cssFileName =
        basename(bundle.displayName, '.html') +
        '.' +
        (options.mode === 'development' ? '[hash]' : hash(stylesheet)) +
        '.css';

      await outputFS.writeFile(join(distDir, cssFileName), stylesheet);

      newContents = (
        await posthtml()
          .use(
            insertAt({
              selector: 'head',
              append:
                '<link href="' +
                bundle.target.publicUrl +
                cssFileName +
                '" rel="stylesheet" />',
              behavior: 'inside',
            }),
          )
          .process(contents.toString())
      ).html;
    }

    return {contents: newContents, map};
  },
});
