import type {SourceLocation} from '@atlaspack/types';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {
  applyCompiledCssInJsPlugin,
  CompiledCssInJsPluginResult,
  type CompiledCssInJsConfig,
} from '@atlaspack/rust/index';
import {join} from 'path';
import SourceMap from '@parcel/source-map';
import type {Diagnostic} from '@atlaspack/diagnostic';
import ThrowableDiagnostic, {
  convertSourceLocationToHighlight,
} from '@atlaspack/diagnostic';
import {remapSourceLocation} from '@atlaspack/utils';

const configFiles = ['.compiledcssrc', '.compiledcssrc.json'];

const PACKAGE_KEY = '@atlaspack/transformer-compiled-css-in-js';

export default new Transformer({
  async loadConfig({config, options}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return {};
    }

    const conf = await config.getConfigFrom<CompiledCssInJsConfig>(
      join(options.projectRoot, 'index'),
      configFiles,
      {
        packageKey: PACKAGE_KEY,
      },
    );

    const contents: CompiledCssInJsConfig = {};

    Object.assign(contents, conf?.contents);

    return contents;
  },
  async transform({asset, options, config, logger}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return [asset];
    }

    const mapPromise = asset.getMap();
    let originalMap: SourceMap | null | undefined;
    const ensureOriginalMap = async () => {
      if (originalMap === undefined) {
        originalMap = await mapPromise;
      }

      return originalMap;
    };
    const code = await asset.getCode();
    if (
      config.importSources?.every(
        (source) =>
          !code.includes(source) || code.includes(source + '/runtime'),
      )
    ) {
      return [asset];
    }

    if (code.includes('cssMap') || /styled[^.]*\.[^`]+`/.test(code)) {
      return [asset];
    }

    const codeBuffer = Buffer.from(code);

    const result = (await applyCompiledCssInJsPlugin(codeBuffer, {
      filename: asset.filePath,
      projectRoot: options.projectRoot,
      isSource: asset.isSource,
      sourceMaps: !!asset.env.sourceMap,
      config,
    })) as CompiledCssInJsPluginResult;

    if (result.diagnostics?.length) {
      const diagnostics = result.diagnostics ?? [];
      asset.meta.compiledCssDiagnostics = JSON.parse(
        JSON.stringify(diagnostics),
      );

      const original = await ensureOriginalMap();
      type PluginDiagnostic = (typeof diagnostics)[number];
      type PluginCodeHighlight = NonNullable<PluginDiagnostic['codeHighlights']>[number];
      type PluginSourceLocation = PluginCodeHighlight['loc'];

      const convertLoc = (loc: PluginSourceLocation): SourceLocation => {
        let location: SourceLocation = {
          filePath: asset.filePath,
          start: {
            line: loc.startLine + Number(asset.meta.startLine ?? 1) - 1,
            column: loc.startCol,
          },
          end: {
            line: loc.endLine + Number(asset.meta.startLine ?? 1) - 1,
            column: loc.endCol,
          },
        };

        if (original) {
          location = remapSourceLocation(
            location,
            original,
            options.projectRoot,
          );
        }

        return location;
      };

      const convertDiagnostic = (diagnostic: PluginDiagnostic): Diagnostic => {
        const codeHighlights = (diagnostic.codeHighlights ?? []).map(
          (highlight: PluginCodeHighlight) =>
            convertSourceLocationToHighlight(
              convertLoc(highlight.loc),
              highlight.message ?? undefined,
            ),
        );

        const converted: Diagnostic = {
          message: diagnostic.message,
          codeFrames: [
            {
              filePath: asset.filePath,
              codeHighlights,
            },
          ],
          hints: diagnostic.hints,
        };

        if (diagnostic.documentationUrl) {
          converted.documentationURL = diagnostic.documentationUrl;
        }

        if (diagnostic.showEnvironment && asset.env.loc) {
          converted.codeFrames?.push({
            filePath: asset.env.loc.filePath,
            codeHighlights: [
              convertSourceLocationToHighlight(
                asset.env.loc,
                'The environment was originally created here',
              ),
            ],
          });
        }

        return converted;
      };

      const errors = diagnostics.filter(
        (diagnostic) =>
          diagnostic.severity === 'Error' ||
          (diagnostic.severity === 'SourceError' && asset.isSource),
      );
      if (errors.length > 0) {
        throw new ThrowableDiagnostic({
          diagnostic: errors.map(convertDiagnostic),
        });
      }

      const warnings = diagnostics.filter(
        (diagnostic) =>
          diagnostic.severity === 'Warning' ||
          (diagnostic.severity === 'SourceError' && !asset.isSource),
      );
      if (warnings.length > 0) {
        logger?.warn(warnings.map(convertDiagnostic));
      }
    }

    if (result.bailOut) {
      return [asset];
    }

    // Handle sourcemap merging if sourcemap is generated
    if (result.map != null) {
      let map = new SourceMap(options.projectRoot);
      map.addVLQMap(JSON.parse(result.map));
      const original = await ensureOriginalMap();
      if (original) {
        // @ts-expect-error TS2345 - the types are wrong, `extends` accepts a `SourceMap` or a `Buffer`
        map.extends(original);
      }
      asset.setMap(map);
    }

    // Rather then setting this as a buffer we set it as a string, since most of the following
    // plugins will call `getCode`, this avoids repeatedly converting the buffer to a string.
    asset.setCode(result.code);

    // Add styleRules to the asset
    asset.meta.styleRules = result.styleRules;

    return [asset];
  },
});
