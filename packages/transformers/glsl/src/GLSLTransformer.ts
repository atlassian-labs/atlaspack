import path from 'path';
import {promisify} from 'util';
import {Transformer} from '@atlaspack/plugin';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'glslify-deps'. '/home/ubuntu/parcel/node_modules/glslify-deps/index.js' implicitly has an 'any' type.
import glslifyDeps from 'glslify-deps';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'glslify-bundle'. '/home/ubuntu/parcel/node_modules/glslify-bundle/index.js' implicitly has an 'any' type.
import glslifyBundle from 'glslify-bundle';

export default new Transformer({
  async transform({asset, resolve}) {
    // Parse and collect dependencies with glslify-deps
    let cwd = path.dirname(asset.filePath);
    let depper = glslifyDeps({
      cwd,
      // @ts-expect-error - TS7006 - Parameter 'target' implicitly has an 'any' type. | TS7006 - Parameter 'opts' implicitly has an 'any' type. | TS7006 - Parameter 'next' implicitly has an 'any' type.
      resolve: async (target, opts, next) => {
        try {
          let filePath = await resolve(
            path.join(opts.basedir, 'index.glsl'),
            target,
          );

          next(null, filePath);
        } catch (err: any) {
          next(err);
        }
      },
    });

    let ast = await promisify(depper.inline.bind(depper))(
      await asset.getCode(),
      cwd,
    );

    collectDependencies(asset, ast);

    // Generate the bundled glsl file
    let glsl = await glslifyBundle(ast);

    asset.setCode(`module.exports=${JSON.stringify(glsl)};`);
    asset.type = 'js';

    return [asset];
  },
}) as Transformer;

function collectDependencies(asset: MutableAsset, ast: any) {
  for (let dep of ast) {
    if (!dep.entry) {
      asset.invalidateOnFileChange(dep.file);
    }
  }
}
