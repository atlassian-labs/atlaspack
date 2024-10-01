import type {State} from './';
import type {REPLOptions} from '../utils';

import fs from 'fs';
import path from 'path';

import {getDefaultTargetEnv} from '../utils';

let commit = fs
  .readFileSync(path.join(__dirname, '../../commit'), 'utf8')
  .trim();

export function Options(
  {
    state,
    dispatch,
    disabled = false,
  }: {
    state: State,
    dispatch: (
      arg1: {
        type: 'options',
        name: keyof REPLOptions,
        value: unknown
      },
    ) => void,
    disabled: boolean | null | undefined
  },
): any {
  const values: REPLOptions = state.options;
  const onChange = (name: keyof REPLOptions, value: unknown) =>
    dispatch({type: 'options', name, value});

  // TODO disabled when watching

  const disablePackageJSON = state.files.has('/package.json');

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="options">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label title="Corresponds to `--no-source-maps`">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Source Maps</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <input
          type="checkbox"
          checked={values.sourceMaps}
// @ts-expect-error - TS2322 - Type 'boolean | null' is not assignable to type 'boolean | undefined'.
          disabled={values.viewSourcemaps || disabled}
          onChange={(e) => onChange('sourceMaps', e.target.checked)}
        />
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label title="Sets `--public-url <value>`">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Public URL</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <input
          type="text"
          value={values.publicUrl}
          placeholder="/"
// @ts-expect-error - TS2339 - Property 'value' does not exist on type 'EventTarget'.
          onInput={(e) => onChange('publicUrl', e.target.value)}
// @ts-expect-error - TS2322 - Type 'boolean | null' is not assignable to type 'boolean | undefined'.
          disabled={disabled}
        />
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Output Format</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <select
          onChange={(e) => onChange('outputFormat', e.target.value || null)}
          value={values.outputFormat ?? ''}
          disabled={disabled || disablePackageJSON}
        >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="" />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="esmodule">esmodule</option>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="commonjs">commonjs</option>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="global">global</option>
        </select>
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Target</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <select
            onChange={(e) => {
              onChange('targetType', e.target.value);
              onChange('targetEnv', null);
            }}
            value={values.targetType}
            style={{marginRight: '0.5rem'}}
            disabled={disabled || disablePackageJSON}
          >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
            <option value="browsers">Browsers</option>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
            <option value="node">Node</option>
          </select>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <input
            type="text"
            value={values.targetEnv ?? ''}
// @ts-expect-error - TS2339 - Property 'value' does not exist on type 'EventTarget'.
            onInput={(e) => onChange('targetEnv', e.target.value || null)}
            placeholder={getDefaultTargetEnv(values.targetType)}
            disabled={disabled || disablePackageJSON}
          />
        </div>
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Mode</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <select
          onChange={(e) => {
            onChange('mode', e.target.value || null);
            if (e.target.value === 'production') {
              onChange('hmr', false);
            } else {
              onChange('scopeHoist', false);
              onChange('minify', false);
            }
          }}
          value={values.mode}
// @ts-expect-error - TS2322 - Type 'boolean | null' is not assignable to type 'boolean | undefined'.
          disabled={disabled}
        >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="production">production</option>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="development">development</option>
        </select>
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>HMR</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <input
          type="checkbox"
          checked={values.hmr}
          onChange={(e) => onChange('hmr', e.target.checked)}
          disabled={disabled || values.mode === 'production'}
        />
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label title="Sets `--no-minify`">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Minify</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <input
          type="checkbox"
          checked={values.minify}
          onChange={(e) => onChange('minify', e.target.checked)}
          disabled={disabled || values.mode === 'development'}
        />
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label title="Corresponds to `--no-scope-hoist`">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Enable Scope Hoisting</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <input
          type="checkbox"
          checked={values.scopeHoist}
          onChange={(e) => onChange('scopeHoist', e.target.checked)}
          disabled={disabled || values.mode === 'development'}
        />
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <hr />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label title="env variable ATLASPACK_DUMP_GRAPHVIZ">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Render Graphs</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <select
          onChange={(e) => onChange('renderGraphs', e.target.value || null)}
// @ts-expect-error - TS2322 - Type 'boolean' is not assignable to type 'string | number | readonly string[] | undefined'.
          value={values.renderGraphs}
// @ts-expect-error - TS2322 - Type 'boolean | null' is not assignable to type 'boolean | undefined'.
          disabled={disabled}
        >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="">disabled</option>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="true">enabled</option>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="symbols">enabled with symbols</option>
        </select>
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <hr />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div className="dependencies">
        Dependencies
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <ul>
          {values.dependencies?.map(([name, version]: [any, any], i: number) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <li key={i}>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
              <input
                type="text"
                value={name}
                placeholder="pkg-name"
                onInput={(e) =>
                  onChange(
                    'dependencies',
                    values.dependencies.map((v, j) =>
// @ts-expect-error - TS2339 - Property 'value' does not exist on type 'EventTarget'.
                      j === i ? [e.target.value, v[1]] : v,
                    ),
                  )
                }
                disabled={disabled || disablePackageJSON}
              />
              @
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
              <input
                value={version}
                placeholder="range"
                onInput={(e) =>
                  onChange(
                    'dependencies',
                    values.dependencies.map((v, j) =>
// @ts-expect-error - TS2339 - Property 'value' does not exist on type 'EventTarget'.
                      j === i ? [v[0], e.target.value] : v,
                    ),
                  )
                }
                disabled={disabled || disablePackageJSON}
              />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
              <button
                className="remove"
                onClick={() =>
                  onChange(
                    'dependencies',
                    values.dependencies.filter((_, j) => j !== i),
                  )
                }
                disabled={disabled || disablePackageJSON}
              >
                ✕
              </button>
            </li>
          ))}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <li>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
            <button
              className="add"
              onClick={() =>
                onChange('dependencies', [...values.dependencies, ['', '']])
              }
              disabled={disabled || disablePackageJSON}
            >
              Add
            </button>
          </li>
        </ul>
      </div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <hr />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <label title="env variable ATLASPACK_WORKERS">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <span>Workers</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <select
          onChange={(e) => onChange('numWorkers', JSON.parse(e.target.value))}
          value={JSON.stringify(values.numWorkers)}
// @ts-expect-error - TS2322 - Type 'boolean | null' is not assignable to type 'boolean | undefined'.
          disabled={disabled}
        >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="0">Use no nested workers</option>
          {navigator.hardwareConcurrency > 0 &&
            new Array(navigator.hardwareConcurrency / 2).fill(0).map((_, i) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
              <option key={i + 1} value={i + 1}>
                Use {i + 1} nested workers
              </option>
            ))}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <option value="null">Default</option>
        </select>
      </label>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div>
        Based on commit{' '}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <a href={`https://github.com/parcel-bundler/parcel/commits/${commit}`}>
          {commit.substr(0, 10)}
        </a>
      </div>
    </div>
  );
}
