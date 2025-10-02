import {Runtime} from '@atlaspack/plugin';
import {loadConfig} from '@atlaspack/utils';
import {version} from 'react-refresh/package.json';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {sort} from '@compiled/css';
import {hash} from '@compiled/utils';
import {Optimizer} from '@atlaspack/plugin';
import posthtml from 'posthtml';
import {transform, browserslistToTargets} from 'lightningcss';
import {insertAt} from 'posthtml-insert-at';
import {basename} from 'path';

interface OptimizerOpts {
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

  lastHash?: string;
}

export default new Runtime<OptimizerOpts>({
  loadConfig({config, options, logger}) {
    return {
      inlineCss: false,
      sortAtRules: true,
      lastHash: undefined,
    };
  },
  apply({config, bundle, bundleGraph, options, logger}) {
    if (bundle.type !== 'js') {
      return;
    }

    console.log('runtime stylesheet', config);

    const styleRules = new Set<string>();

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
      message: 'runtime styleRules ' + Array.from(styleRules).join('\n'),
    });

    const sortConfig = {
      sortAtRulesEnabled: config.sortAtRules,
      sortShorthandEnabled: config.sortShorthand,
    };
    const stylesheet = sort(Array.from(styleRules).join(''), sortConfig);

    const hashed = hash(stylesheet);

    if (config?.lastHash === hashed) {
      // Skip if the hash is the same as the last hash
      return;
    }
    config.lastHash = hashed;

    const code = `
        const style = document.querySelector('style#cmpl-atlaspack-dev') ?? (document.createElement('style'));
        style.id = 'cmpl-atlaspack-dev';
        style.textContent = \`${stylesheet
          .toString()
          .replaceAll('`', '\\`')
          .replaceAll('\n', ' ')}\`;
        document.querySelector('head').appendChild(style);
    `;

    return {
      filePath: __filename,
      code,
      isEntry: true,
    };
  },
});
