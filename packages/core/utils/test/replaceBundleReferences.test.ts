import type {NamedBundle, Dependency} from '@atlaspack/types';

import assert from 'assert';
import {getURLReplacement} from '../src/replaceBundleReferences';

describe('replace bundle references', () => {
  it('Query params and named pipeline, relative', () => {
    let fromBundle: NamedBundle = {
      filePath: '/user/dist/reformat.html',
      name: 'reformat.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    let toBundle: NamedBundle = {
      filePath:
        '/user/dist/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      name: 'image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    // @ts-expect-error - TS2740 - Type '{ id: string; specifier: string; specifierType: "esm"; }' is missing the following properties from type 'Dependency': priority, bundleBehavior, needsStableName, isOptional, and 13 more.
    let dependency: Dependency = {
      id: '074b36596e3147e900a8ad17ceb5c90b',
      specifier: 'url:./image.jpg?as=webp',
      specifierType: 'esm',
    };

    let result = getURLReplacement({
      dependency,
      fromBundle,
      toBundle,
      relative: true,
    });

    assert.equal(
      result.to,
      'image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
    );
    assert.equal(result.from, '074b36596e3147e900a8ad17ceb5c90b');
  });

  it('Query params and named pipeline, absolute', () => {
    let fromBundle: NamedBundle = {
      filePath: '/user/dist/reformat.html',
      name: 'reformat.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    let toBundle: NamedBundle = {
      filePath:
        '/user/dist/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      name: 'image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    // @ts-expect-error - TS2740 - Type '{ id: string; specifier: string; specifierType: "esm"; }' is missing the following properties from type 'Dependency': priority, bundleBehavior, needsStableName, isOptional, and 13 more.
    let dependency: Dependency = {
      id: '074b36596e3147e900a8ad17ceb5c90b',
      specifier: 'url:./image.jpg?as=webp',
      specifierType: 'esm',
    };

    let result = getURLReplacement({
      dependency,
      fromBundle,
      toBundle,
      relative: false,
    });

    assert.equal(
      result.to,
      '/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
    );
    assert.equal(result.from, '074b36596e3147e900a8ad17ceb5c90b');
  });

  it('Custom Public URL', () => {
    let fromBundle: NamedBundle = {
      filePath: '/user/dist/reformat.html',
      name: 'reformat.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: 'https://test.com/static',
      },
    };

    let toBundle: NamedBundle = {
      filePath:
        '/user/dist/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      name: 'image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: 'https://test.com/static',
      },
    };

    // @ts-expect-error - TS2740 - Type '{ id: string; specifier: string; specifierType: "esm"; }' is missing the following properties from type 'Dependency': priority, bundleBehavior, needsStableName, isOptional, and 13 more.
    let dependency: Dependency = {
      id: '074b36596e314797845a8ad17ceb5c9b',
      specifier: './image.jpg',
      specifierType: 'esm',
    };

    let result = getURLReplacement({
      dependency,
      fromBundle,
      toBundle,
      relative: false,
    });

    assert.equal(
      result.to,
      'https://test.com/static/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
    );
    assert.equal(result.from, '074b36596e314797845a8ad17ceb5c9b');
  });

  it('Relative with folders in between', () => {
    let fromBundle: NamedBundle = {
      filePath: '/user/dist/reformat.html',
      name: 'reformat.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: 'https://test.com/static',
      },
    };

    let toBundle: NamedBundle = {
      filePath:
        '/user/dist/assets/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      name: 'image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist/assets',
        publicUrl: 'https://test.com/static',
      },
    };

    // @ts-expect-error - TS2740 - Type '{ id: string; specifier: string; specifierType: "esm"; }' is missing the following properties from type 'Dependency': priority, bundleBehavior, needsStableName, isOptional, and 13 more.
    let dependency: Dependency = {
      id: '074b36596e3147e900a8ad17ceb5c90b',
      specifier: 'url:./image.jpg?as=webp',
      specifierType: 'esm',
    };

    let result = getURLReplacement({
      dependency,
      fromBundle,
      toBundle,
      relative: true,
    });

    assert.equal(
      result.to,
      'assets/image.HASH_REF_87f9d66c16c2216ccc7e5664cf089305.webp',
    );
    assert.equal(result.from, '074b36596e3147e900a8ad17ceb5c90b');
  });

  it('should work with bundle names with colons, relative', () => {
    let fromBundle: NamedBundle = {
      filePath: '/user/dist/reformat.html',
      name: 'reformat.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    let toBundle: NamedBundle = {
      filePath: '/user/dist/a:b:c.html',
      name: 'a:b:c.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    // @ts-expect-error - TS2740 - Type '{ id: string; specifier: string; specifierType: "esm"; }' is missing the following properties from type 'Dependency': priority, bundleBehavior, needsStableName, isOptional, and 13 more.
    let dependency: Dependency = {
      id: '074b36596e3147e900a8ad17ceb5c90b',
      specifier: './a:b:c.html',
      specifierType: 'esm',
    };

    let result = getURLReplacement({
      dependency,
      fromBundle,
      toBundle,
      relative: true,
    });

    assert.equal(result.to, './a:b:c.html');
  });

  it('should work with bundle names with colons, absolute', () => {
    let fromBundle: NamedBundle = {
      filePath: '/user/dist/reformat.html',
      name: 'reformat.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    let toBundle: NamedBundle = {
      filePath: '/user/dist/a:b:c.html',
      name: 'a:b:c.html',
      // $FlowFixMe
      // @ts-expect-error - TS2739 - Type '{ distDir: string; publicUrl: string; }' is missing the following properties from type 'Target': distEntry, env, name, loc
      target: {
        distDir: '/user/dist',
        publicUrl: '/',
      },
    };

    // @ts-expect-error - TS2740 - Type '{ id: string; specifier: string; specifierType: "esm"; }' is missing the following properties from type 'Dependency': priority, bundleBehavior, needsStableName, isOptional, and 13 more.
    let dependency: Dependency = {
      id: '074b36596e3147e900a8ad17ceb5c90b',
      specifier: './a:b:c.html',
      specifierType: 'esm',
    };

    let result = getURLReplacement({
      dependency,
      fromBundle,
      toBundle,
      relative: false,
    });

    assert.equal(result.to, '/a:b:c.html');
  });
});
