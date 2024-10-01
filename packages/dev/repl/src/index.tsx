import {Fragment, useEffect, useState, useReducer, useRef} from 'react';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'react-dom/client'. '/home/ubuntu/parcel/packages/dev/repl/node_modules/react-dom/client.js' implicitly has an 'any' type.
import {createRoot} from 'react-dom/client';
import {Panel, PanelGroup, PanelResizeHandle} from 'react-resizable-panels';
import {useMedia} from 'react-use';

// @ts-expect-error - TS2307 - Cannot find module 'url:./assets/logo.svg' or its corresponding type declarations.
import parcelLogo from 'url:./assets/logo.svg';
// @ts-expect-error - TS2307 - Cannot find module 'url:./assets/parcel.png' or its corresponding type declarations.
import parcelText from 'url:./assets/parcel.png';

import {
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'Editor'.
  Editor,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'FileBrowser'.
  FileBrowser,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'Notes'.
  Notes,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'Options'.
  Options,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'ParcelError'.
  ParcelError,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'PresetSelector'.
  PresetSelector,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'Preview'.
  Preview,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'Tabs'.
  Tabs,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'Graphs'.
  Graphs,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'useDebounce'.
  useDebounce,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'useKeyboard'.
  useKeyboard,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'usePromise'.
  usePromise,
// @ts-expect-error - TS2305 - Module '"./components/"' has no exported member 'useSessionStorage'.
  useSessionStorage,
} from './components/';
import {saveState, reducer, getInitialState} from './components';
import type {State} from './components';
import filesize from 'filesize';
import {linkSourceMapVisualization} from './utils';
import nullthrows from 'nullthrows';

import {
  bundle,
  watch,
  workerReady,
  waitForFS,
  clientID as clientIDPromise,
} from './atlaspack/';

const STATUS_LOADING = Symbol('STATUS_LOADING');
const STATUS_RUNNING = Symbol('STATUS_RUNNING');
const STATUS_IDLING = Symbol('STATUS_IDLING');

// @ts-expect-error - TS7031 - Binding element 'watching' implicitly has an 'any' type. | TS7031 - Binding element 'status' implicitly has an 'any' type. | TS7031 - Binding element 'buildProgress' implicitly has an 'any' type. | TS7031 - Binding element 'buildOutput' implicitly has an 'any' type.
function Status({watching, status, buildProgress, buildOutput}) {
  let buildDuration =
    buildOutput?.buildTime != null
      ? Math.round(buildOutput?.buildTime / 10) / 100
      : null;

  let text, color;
  if (status === STATUS_LOADING) {
    text = 'Loading...';
    color = '#D97706';
    // color = '#553701';
  } else if (status === STATUS_IDLING) {
    if (watching) {
      if (buildDuration != null) {
        text = `Watching... (last build took ${buildDuration}s)`;
      } else {
        text = 'Watching...';
      }
    } else {
      if (buildDuration != null) {
        text = `Finished in ${buildDuration}s`;
      } else {
        text = 'Ready';
      }
    }
    color = '#059669';
    // color = '#015551';
    // TODO: errors + "finished in 123s"
  } else if (status === STATUS_RUNNING) {
    if (buildProgress) {
      text = 'Running: ' + buildProgress;
    } else {
      text = 'Running...';
    }
    color = '#ffeb3b';
  }

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="status" style={{backgroundColor: color}}>
      {text}
    </div>
  );
}

function Output({
  state,
  dispatch,
}: {
  state: State,
  dispatch: any
}) {
  let [watching, setWatching] = useState(false);
  let [buildState, setBuildState] = useState(STATUS_LOADING);
  let [buildOutput, setBuildOutput] = useState(null);
  let [buildProgress, setBuildProgress] = useState(null);
  let [outputTabIndex, setOutputTabIndex] = useSessionStorage(
    'outputTabIndex',
    0,
  );
  let watchSubscriptionRef = useRef(null);

  useEffect(() => {
    setBuildState(STATUS_LOADING);
    workerReady(state.options.numWorkers).then(() => {
// @ts-expect-error - TS2345 - Argument of type 'unique symbol' is not assignable to parameter of type 'SetStateAction<unique symbol>'.
      setBuildState(STATUS_IDLING);
    });
  }, [state.options.numWorkers]);

  async function build() {
// @ts-expect-error - TS2345 - Argument of type 'unique symbol' is not assignable to parameter of type 'SetStateAction<unique symbol>'.
    setBuildState(STATUS_RUNNING);

    setBuildProgress(null);

    try {
// @ts-expect-error - TS2345 - Argument of type 'Dispatch<SetStateAction<null>>' is not assignable to parameter of type '(arg1: string) => void'.
      const output = await bundle(state.files, state.options, setBuildProgress);

// @ts-expect-error - TS2345 - Argument of type 'BundleOutput' is not assignable to parameter of type 'SetStateAction<null>'.
      setBuildOutput(output);
      dispatch({
        type: 'diagnostics',
        value:
          output.type === 'failure' && output.diagnostics
            ? new Map(
                [...output.diagnostics]
// @ts-expect-error - TS2769 - No overload matches this call.
                  .filter(([name]: [any]) => name)
                  .map(([name, data]: [any, any]) => ['/' + name, data]),
              )
            : null,
      });
    } catch (error: any) {
      console.error('Unexpected error', error);
    }

// @ts-expect-error - TS2345 - Argument of type 'unique symbol' is not assignable to parameter of type 'SetStateAction<unique symbol>'.
    setBuildState(STATUS_IDLING);
  }

  async function toggleWatch() {
    if (watchSubscriptionRef.current) {
// @ts-expect-error - TS2339 - Property 'unsubscribe' does not exist on type 'never'.
      watchSubscriptionRef.current.unsubscribe();
      watchSubscriptionRef.current = null;
      setWatching(false);
    } else {
      setWatching(true);
// @ts-expect-error - TS2345 - Argument of type 'unique symbol' is not assignable to parameter of type 'SetStateAction<unique symbol>'.
      setBuildState(STATUS_RUNNING);
      let {unsubscribe, writeAssets} = await watch(
        state.files,
        state.options,
        (output) => {
// @ts-expect-error - TS2345 - Argument of type 'unique symbol' is not assignable to parameter of type 'SetStateAction<unique symbol>'.
          setBuildState(STATUS_IDLING);
// @ts-expect-error - TS2345 - Argument of type 'BundleOutput' is not assignable to parameter of type 'SetStateAction<null>'.
          setBuildOutput(output);
          dispatch({
            type: 'diagnostics',
            value:
              output.type === 'failure' && output.diagnostics
                ? new Map(
                    [...output.diagnostics]
// @ts-expect-error - TS2769 - No overload matches this call.
                      .filter(([name]: [any]) => name)
                      .map(([name, data]: [any, any]) => ['/' + name, data]),
                  )
                : null,
          });
        },
// @ts-expect-error - TS2345 - Argument of type 'Dispatch<SetStateAction<null>>' is not assignable to parameter of type '(arg1?: string | null | undefined) => void'.
        setBuildProgress,
      );
// @ts-expect-error - TS2322 - Type '{ unsubscribe: () => Promise<unknown>; writeAssets: (arg1: FS) => Promise<unknown>; }' is not assignable to type 'null'.
      watchSubscriptionRef.current = {unsubscribe, writeAssets};
    }
  }

  useEffect(() => {
    if (watchSubscriptionRef.current) {
// @ts-expect-error - TS2339 - Property 'writeAssets' does not exist on type 'never'.
      watchSubscriptionRef.current.writeAssets(state.files);
// @ts-expect-error - TS2345 - Argument of type 'unique symbol' is not assignable to parameter of type 'SetStateAction<unique symbol>'.
      setBuildState(STATUS_RUNNING);
    }
  }, [state.files]);

  useKeyboard(
// @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
    (e) => {
      if (
        e.metaKey &&
        e.code === 'KeyB' &&
        !watching &&
        buildState !== STATUS_RUNNING
      ) {
        build();
        e.preventDefault();
      }
    },
    [build, buildState, watching],
  );

  let [clientID] = usePromise(clientIDPromise);

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="output">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <Status
        watching={watching}
        status={buildState}
        buildProgress={buildProgress}
        buildOutput={buildOutput}
      />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div className="header">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <button
          disabled={watching || buildState !== STATUS_IDLING}
          onClick={build}
        >
          Build
        </button>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <button disabled={buildState !== STATUS_IDLING} onClick={toggleWatch}>
          {watching ? 'Stop watching' : 'Watch'}
        </button>
      </div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div className="files">
{ /* @ts-expect-error - TS2339 - Property 'type' does not exist on type 'never'. */}
        {buildOutput?.type === 'success' && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
          <Tabs
            names={['Output', 'Preview']}
            selected={outputTabIndex}
            setSelected={setOutputTabIndex}
          >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
            <div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
              <div className="list views">
{ /* @ts-expect-error - TS2339 - Property 'bundles' does not exist on type 'never'. | TS7031 - Binding element 'name' implicitly has an 'any' type. | TS7031 - Binding element 'size' implicitly has an 'any' type. | TS7031 - Binding element 'content' implicitly has an 'any' type. */}
                {buildOutput.bundles.map(({name, size, content}) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
                  <div key={name} className="view selected">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
                    <div className="name">
                      {content.length < 500000 &&
// @ts-expect-error - TS2531 - Object is possibly 'null'.
                      buildOutput.sourcemaps?.has(name) ? (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
                        <a
                          href="https://evanw.github.io/source-map-visualization/#"
                          target="_blank"
                          rel="noopener noreferrer"
                          onClick={(event) => {
// @ts-expect-error - TS2339 - Property 'href' does not exist on type 'EventTarget'.
                            event.target.href = linkSourceMapVisualization(
                              content,
// @ts-expect-error - TS2531 - Object is possibly 'null'.
                              nullthrows(buildOutput.sourcemaps?.get(name)),
                            );
                          }}
                        >
                          Map
                        </a>
                      ) : (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
                        <span />
                      )}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
                      <span>{name}</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
                      <span>{filesize(size)}</span>
                    </div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
                    <Editor name={name} value={content} readOnly />
                  </div>
                ))}
              </div>
{ /* @ts-expect-error - TS2339 - Property 'graphs' does not exist on type 'never'. | TS17004 - Cannot use JSX unless the '--jsx' flag is provided. | TS2339 - Property 'graphs' does not exist on type 'never'. */}
              {buildOutput?.graphs && <Graphs graphs={buildOutput.graphs} />}
            </div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
            <Preview clientID={waitForFS().then(() => nullthrows(clientID))} />
          </Tabs>
        )}
{ /* @ts-expect-error - TS2339 - Property 'type' does not exist on type 'never'. */}
        {buildOutput?.type === 'failure' && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
          <ParcelError output={buildOutput} />
        )}
      </div>
    </div>
  );
}

// @ts-expect-error - TS7031 - Binding element 'state' implicitly has an 'any' type. | TS7031 - Binding element 'dispatch' implicitly has an 'any' type.
function Editors({state, dispatch}) {
  const views = [...state.views];
  const names = views.map(([name, data]: [any, any]) => (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <Fragment key={name}>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <span></span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <span>{name}</span>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <button
        className={
          'close ' +
          (data.value !== state.files.get(name)?.value ? 'modified' : '')
        }
        onClick={() => dispatch({type: 'view.close', name})}
      ></button>
    </Fragment>
  ));
  const children = views.map(([name, data]: [any, any]) => {
    if (data.component) {
      let Comp = data.component;
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
      return <Comp key={name} state={state} dispatch={dispatch} />;
    } else {
      return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
        <Editor
          key={name}
          dispatch={dispatch}
          name={name}
          value={data.value}
          diagnostics={state.diagnostics.get(name)}
        />
      );
    }
  });

  if (state.useTabs) {
    return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
      <Tabs
        names={names}
        className="editors views"
        mode="hide"
        selected={state.currentView}
// @ts-expect-error - TS7006 - Parameter 'i' implicitly has an 'any' type.
        setSelected={(i) => dispatch({type: 'view.select', index: i})}
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
        fallback={<Notes />}
      >
        {children}
      </Tabs>
    );
  } else {
    let merged: Array<React.ReactElement<React.ComponentProps<'div'>>> = [];
    for (let i = 0; i < views.length; i++) {
      merged.push(
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
        <div className="view" key={i}>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <div className="name selected">{names[i]}</div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <div className="content">{children[i]}</div>
        </div>,
      );
    }
    return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
      <div className="list editors views">
        {merged}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        {children.length === 0 && <Notes />}
      </div>
    );
  }
}

function App() {
  let [state, dispatch] = useReducer(reducer, null, getInitialState);

  let isDesktop = useMedia('(min-width: 800px)');

  useDebounce(() => saveState(state), 500, [state.files, state.options]);

  useKeyboard(
// @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
    (e) => {
      if (e.metaKey && e.code === 'KeyS') {
        dispatch({type: 'view.saveCurrent'});
        e.preventDefault();
      } else if (e.ctrlKey && e.code === 'KeyW') {
        dispatch({type: 'view.closeCurrent'});
        e.preventDefault();
      }
    },
    [dispatch],
  );

  const sidebar = (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <FileBrowser
      files={state.files}
      collapsed={state.browserCollapsed}
      dispatch={dispatch}
      isEditing={state.isEditing}
    >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <header>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <a href="/">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <img
            className="parcel"
            src={parcelText}
            height="30"
            style={{marginTop: '5px'}}
            alt=""
          />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <img
            className="type"
            src={parcelLogo}
            style={{width: '120px'}}
            alt=""
          />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <span style={{fontSize: '25px'}}>REPL</span>
        </a>
      </header>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <PresetSelector dispatch={dispatch} />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <div className="options">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <button
            onClick={() =>
              dispatch({
                type: 'view.open',
                name: 'Options',
                component: Options,
              })
            }
          >
            Options
          </button>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <button
            title="Toggle view"
            className={'view ' + (state.useTabs ? 'tabs' : '')}
            onClick={() =>
              dispatch({
                type: 'toggleView',
              })
            }
          >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
            <span></span>
          </button>
        </div>
      </div>
    </FileBrowser>
  );

// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
  const editors = <Editors state={state} dispatch={dispatch} />;
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
  const output = <Output state={state} dispatch={dispatch} />;

  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <main>
      {isDesktop ? (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
        <PanelGroup direction="horizontal" autoSaveId="repl-main-panels">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <Panel
            defaultSizePercentage={20}
            minSizePixels={60}
            className="panel"
          >
            {sidebar}
          </Panel>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <ResizeHandle />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <Panel
            defaultSizePercentage={45}
            minSizePixels={100}
            className="panel"
          >
            {editors}
          </Panel>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <ResizeHandle />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <Panel
            defaultSizePercentage={35}
            minSizePixels={200}
            className="panel"
          >
            {output}
          </Panel>
        </PanelGroup>
      ) : (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
        <div style={{display: 'flex', flexDirection: 'column'}}>
          {sidebar}
          {editors}
          {output}
        </div>
      )}
    </main>
  );
}

function ResizeHandle() {
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
  return <PanelResizeHandle className="resize-handle"></PanelResizeHandle>;
}

let root = createRoot(document.getElementById('root'));
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
root.render(<App />);

if (navigator.serviceWorker) {
  navigator.serviceWorker
    // $FlowFixMe
// @ts-expect-error - TS1343 - The 'import.meta' meta-property is only allowed when the '--module' option is 'es2020', 'es2022', 'esnext', 'system', 'node12', or 'nodenext'.
    .register(new URL('./sw.js', import /*:: ("") */.meta.url), {
      type: 'module',
    })
    .catch((error) => {
      console.error('Service worker registration failed:', error);
    });
}
