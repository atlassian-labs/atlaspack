import type {BundleOutputError} from '../atlaspack/AtlaspackWorker';
import {useCallback, useState, useEffect, useRef, memo} from 'react';
import {ctrlKey} from '../utils';
import renderGraph from '../graphs/index';
import {ASSET_PRESETS, extractZIP} from '../utils';
import {FSMap} from '../utils/assets';
/* eslint-disable react/jsx-no-bind */

export function ParcelError(
  {
    output: {error},
  }: {
    output: BundleOutputError
  },
): any {
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="build-error">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <span>A build error occured:</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div className="content" dangerouslySetInnerHTML={{__html: error}} />
    </div>
  );
}

export function Notes(): any {
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="help">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <p>{ctrlKey} + B to bundle</p>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <p>{ctrlKey} + S to save</p>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <p>Ctrl + W to close a tab</p>
        {/* Note:
        <ul>
          <li>
            PostHTML&apos;s <code>removeUnusedCss</code> is disabled for a
            smaller bundle size
          </li>
        </ul>
        <br />
        Based on commit:{' '}
        <a href={`https://github.com/parcel-bundler/parcel/tree/${commit}`}>
          {commit}
        </a> */}
      </div>
    </div>
  );
}

// function toDataURI(mime, data) {
//   return `data:${mime};charset=utf-8;base64,${btoa(data)}`;
// }

// @ts-expect-error - TS2339 - Property 'graphs' does not exist on type '{ children?: ReactNode; }'.
export const Graphs: any = memo(function Graphs({graphs}) {
  let [rendered, setRendered] = useState();

  useEffect(() => {
    renderGraph().then(async (render) => {
      setRendered(
        await Promise.all(
// @ts-expect-error - TS7031 - Binding element 'name' implicitly has an 'any' type. | TS7031 - Binding element 'content' implicitly has an 'any' type.
          graphs.map(async ({name, content}) => ({
            name,
// @ts-expect-error - TS2695 - Left side of comma operator is unused and has no side effects.
            content: /*toDataURI*/ ('image/svg+xml', await render(content)),
          })),
        ),
      );
    });
  }, [graphs]);

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="graphs">
      Graphs (will open in a new tab)
      {rendered && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
        <div>
{ /* @ts-expect-error - TS2339 - Property 'map' does not exist on type 'never'. | TS7031 - Binding element 'name' implicitly has an 'any' type. | TS7031 - Binding element 'content' implicitly has an 'any' type. | TS7006 - Parameter 'i' implicitly has an 'any' type. */}
          {rendered.map(({name, content}, i) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <button
              key={i}
              onClick={() => {
                var win = window.open();
// @ts-expect-error - TS2531 - Object is possibly 'null'.
                win.document.write(content);
                // win.document.write(
                //   '<iframe src="' +
                //     content +
                //     '" frameborder="0" style="border:0; top:0px; left:0px; bottom:0px; right:0px; width:100%; height:100%;" allowfullscreen></iframe>',
                // );
              }}
            >
              {name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
});

export function Tabs(
  {
    names,
    children,
    selected,
    setSelected,
    mode = 'remove',
    className,
    fallback,
    ...props
  }: any,
): any {
  let [_selected, _setSelected] = useState(0);

  selected = selected ?? _selected;
  setSelected = setSelected ?? _setSelected;

  useEffect(() => {
    if (children.length > 0 && children.length <= selected) {
      setSelected(selected - 1);
    }
  }, [children, selected, setSelected]);

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div {...props} className={'tabs ' + (className || '')}>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div className="switcher">
{ /* @ts-expect-error - TS7006 - Parameter 'n' implicitly has an 'any' type. | TS7006 - Parameter 'i' implicitly has an 'any' type. */}
        {names.map((n, i) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
          <div
            onClick={() => setSelected(i)}
            key={i}
            className={i === selected ? 'selected' : undefined}
            // tabIndex="0"
            // onKeyDown={(e) => e.code === "Enter" && setSelected(i)}
          >
            {n}
          </div>
        ))}
      </div>
      {mode === 'remove'
// @ts-expect-error - TS7006 - Parameter '_' implicitly has an 'any' type. | TS7006 - Parameter 'i' implicitly has an 'any' type.
        ? children.find((_, i) => i === selected)
// @ts-expect-error - TS7006 - Parameter 'c' implicitly has an 'any' type. | TS7006 - Parameter 'i' implicitly has an 'any' type.
        : children.map((c, i) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <div
              key={i}
              className="content"
              style={i !== selected ? {display: 'none'} : undefined}
            >
              {c}
            </div>
          ))}
      {children.length === 0 && fallback}
    </div>
  );
}

export function EditableField(
  {
    value,
    editing,
    onChange,
  }: any,
): any {
  let [v, setV] = useState(value);

  useEffect(() => {
    if (editing) {
      let handler = () => {
        onChange(v);
      };

      window.addEventListener('click', handler);

      return () => {
        window.removeEventListener('click', handler);
      };
    }
  }, [v, editing, onChange]);

  useEffect(() => {
    if (editing) {
      setV(value);
    }
  }, [editing, value]);

  return editing ? (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onChange(v);
      }}
      style={{display: 'inline'}}
    >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <input
        type="text"
        value={v}
        onInput={(e) => {
// @ts-expect-error - TS2339 - Property 'value' does not exist on type 'EventTarget'.
          setV(e.target.value);
        }}
        onClick={(e) => e.stopPropagation()}
      />
    </form>
  ) : (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <span>{value}</span>
  );
}

export function PresetSelector(
  {
    dispatch,
  }: any,
): any {
  let onChange = useCallback(
    async (preset) => {
      if (preset === 'Three.js Benchmark') {
        try {
          let data = await (
// @ts-expect-error - TS1343 - The 'import.meta' meta-property is only allowed when the '--module' option is 'es2020', 'es2022', 'esnext', 'system', 'node12', or 'nodenext'.
            await fetch(new URL('../assets/three.zip', import.meta.url))
          ).arrayBuffer();
          let files: FSMap = await extractZIP(data);

          let fs = new Map([
            ['copy1', files],
            ['copy2', files],
            ['copy3', files],
            [
              'index.js',
              {
// @ts-expect-error - TS2769 - No overload matches this call.
                isEntry: true,
                value: `import * as copy1 from './copy1/Three.js'; export {copy1};
        import * as copy2 from './copy2/Three.js'; export {copy2};
        import * as copy3 from './copy3/Three.js'; export {copy3};`,
              },
            ],
          ]);

          dispatch({type: 'preset.load', name: preset, data: {fs}});
        } catch (e: any) {
          console.error(e);
        }
      } else {
        dispatch({type: 'preset.load', name: preset});
      }
    },
    [dispatch],
  );

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <label className="presets">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <span>Preset:</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <select
        onChange={(e) => {
          onChange(e.target.value);
        }}
        value={''}
      >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <option value=""></option>
        {[...ASSET_PRESETS.keys()].map((n) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
          <option key={n} value={n}>
            {n}
          </option>
        ))}
      </select>
    </label>
  );
}
// ----------------------------------------------------------------------------------------

export function useDebounce(cb: () => unknown, delay: number, deps: Array<unknown>): any {
  useEffect(() => {
    const handler = setTimeout(() => {
      cb();
    }, delay);

    return () => {
      clearTimeout(handler);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cb, delay, ...deps]);
}

export function useSessionStorage(key: string, initialValue: unknown): [any, () => void] {
  const [storedValue, setStoredValue] = useState(() => {
    try {
      const item = window.sessionStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch (error: any) {
      console.log(error);
      return initialValue;
    }
  });

  const setValue = (value: undefined) => {
    try {
      const valueToStore =
// @ts-expect-error - TS2358 - The left-hand side of an 'instanceof' expression must be of type 'any', an object type or a type parameter. | TS2349 - This expression is not callable.
        value instanceof Function ? value(storedValue) : value;
      setStoredValue(valueToStore);
      window.sessionStorage.setItem(key, JSON.stringify(valueToStore));
    } catch (error: any) {
      console.log(error);
    }
  };

// @ts-expect-error - TS2322 - Type '(value: undefined) => void' is not assignable to type '() => void'.
  return [storedValue, setValue];
}

export function usePromise<T>(promise: Promise<T>): [T | null | undefined, any, boolean] {
  let [state, setState] = useState(null);
  let mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    promise.then(
// @ts-expect-error - TS2345 - Argument of type '{ resolved: T; }' is not assignable to parameter of type 'SetStateAction<null>'.
      (v) => mountedRef.current && setState({resolved: v}),
// @ts-expect-error - TS2345 - Argument of type '{ rejected: any; }' is not assignable to parameter of type 'SetStateAction<null>'.
      (v) => mountedRef.current && setState({rejected: v}),
    );
  }, [promise]);

// @ts-expect-error - TS2339 - Property 'resolved' does not exist on type 'never'. | TS2339 - Property 'rejected' does not exist on type 'never'.
  return [state?.resolved, state?.rejected, state != null];
}

const addBodyClass = (className: string) => document.body.classList.add(className);
const removeBodyClass = (className: string) =>
  document.body.classList.remove(className);
export function useBodyClass(className: string) {
  useEffect(() => {
    let classNames = Array.isArray(className) ? className : [className];
    classNames.forEach(addBodyClass);

    return () => {
      classNames.forEach(removeBodyClass);
    };
  }, [className]);
}

export function useKeyboard(cb: (arg1: KeyboardEvent) => unknown, deps: Array<unknown>) {
  const keydownCb = useCallback(
    (e: KeyboardEvent) => {
      cb(e);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [cb, ...deps],
  );
  useEffect(() => {
    document.addEventListener('keydown', keydownCb);
    return () => {
      document.removeEventListener('keydown', keydownCb);
    };
  }, [keydownCb]);
}
