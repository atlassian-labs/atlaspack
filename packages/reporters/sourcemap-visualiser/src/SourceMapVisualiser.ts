import path from 'path';
import nullthrows from 'nullthrows';
import {Reporter} from '@atlaspack/plugin';
import {relativePath} from '@atlaspack/utils';

export default new Reporter({
  async report({event, options, logger}) {
    if (event.type === 'buildSuccess') {
      let bundles: Array<{
        content: string;
        mappings: any;
        name: FilePath;
        names: any;
        sources: never;
      }> = [];
      for (let bundle of event.bundleGraph.getBundles()) {
        let p = bundle.filePath;
        if (p) {
          let mapFilePath = p + '.map';
          let hasMap = await options.outputFS.exists(mapFilePath);
          if (hasMap) {
            let map = JSON.parse(
              await options.outputFS.readFile(mapFilePath, 'utf-8'),
            );

            let mappedSources = await Promise.all(
              // @ts-expect-error - TS7006 - Parameter 'sourceName' implicitly has an 'any' type. | TS7006 - Parameter 'index' implicitly has an 'any' type.
              map.sources.map(async (sourceName, index) => {
                let sourceContent = map.sourcesContent?.[index];
                if (sourceContent != null) {
                  try {
                    sourceContent = await options.inputFS.readFile(
                      path.resolve(options.projectRoot, sourceName),
                      'utf-8',
                    );
                  } catch (e: any) {
                    logger.warn({
                      message: `Error while loading content of ${sourceName}, ${e.message}`,
                    });
                  }
                }

                return {
                  name: sourceName,
                  content: sourceContent ?? '',
                };
              }),
            );

            let fileName = relativePath(options.projectRoot, p);
            bundles.push({
              name: fileName,
              mappings: map.mappings,
              names: map.names,
              // @ts-expect-error - TS2322 - Type 'any[]' is not assignable to type 'never'.
              sources: mappedSources,
              content: await options.outputFS.readFile(
                nullthrows(bundle.filePath),
                'utf-8',
              ),
            });
          }
        }
      }

      await options.outputFS.writeFile(
        path.join(options.projectRoot, 'sourcemap-info.json'),
        JSON.stringify(bundles),
      );

      logger.log({
        message: `Goto https://sourcemap-visualiser.now.sh/ and upload the generated sourcemap-info.json file to visualise and debug the sourcemaps.`,
      });
    }
  },
}) as Reporter;
