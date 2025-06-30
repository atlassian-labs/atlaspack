// @flow

/**
 * @description
 * This is used to tell babel if it should resolve "./lib" or "./src".
 * It will be replaced with Nodejs conditional exports
 * */
export const USE_LIB = process.env.ATLASPACK_REGISTER_USE_LIB === 'true';
