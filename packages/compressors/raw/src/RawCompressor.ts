import {Compressor} from '@atlaspack/plugin';

export default new Compressor({
  compress({stream}) {
    return {stream};
  },
}) as Compressor;