import {Transformer} from '@atlaspack/plugin';
// @ts-expect-error - TS7016 - Could not find a declaration file for module '@mdx-js/mdx'. '/home/ubuntu/parcel/node_modules/@mdx-js/mdx/index.js' implicitly has an 'any' type.
import mdx from '@mdx-js/mdx';

export default new Transformer({
  async transform({asset}) {
    let code = await asset.getCode();
    let compiled = await mdx(code);

    asset.type = 'js';
    asset.setCode(`/* @jsxRuntime classic */
/* @jsx mdx */
import React from 'react';
import { mdx } from '@mdx-js/react'
${compiled}
`);

    return [asset];
  },
}) as Transformer;
