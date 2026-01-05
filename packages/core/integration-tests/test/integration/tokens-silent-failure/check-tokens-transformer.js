// This transformer simulates the Compiled CSS transformer behavior:
// it expects no tokens to remain in the file after the tokens transformer runs.
// If it finds any token() calls, it throws an error.

const {Transformer} = require('@atlaspack/plugin');

module.exports = new Transformer({
  async transform({asset}) {
    const code = await asset.getCode();
    // Check if there are any token() calls remaining in the code
    // This simulates what the Compiled CSS transformer expects
    if (code.includes('token(')) {
      throw new Error(
        `Found untransformed token() call in ${asset.filePath}. ` +
        `The tokens transformer should have transformed all tokens, but some remain. ` +
        `This indicates a silent failure in the tokens transformer.`
      );
    }

    return [asset];
  }
});

