import {Fragment, useEffect, useState, useReducer, useRef} from 'react';
// @ts-expect-error TS7016
import {createRoot} from 'react-dom/client';
import {Panel, PanelGroup, PanelResizeHandle} from 'react-resizable-panels';
import {useMedia} from 'react-use';

// @ts-expect-error TS2307
import parcelLogo from 'url:./assets/logo.svg';
// @ts-expect-error TS2307
import parcelText from 'url:./assets/atlaspack.png';

import {
  Editor,
  FileBrowser,
  Notes,
  Options,
  ParcelError,
  PresetSelector,
  Preview,
  Tabs,
  Graphs,
  useDebounce,
  useKeyboard,
  usePromise,
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

// @ts-expect-error TS7031
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
    // @ts-expect-error TS17004
    <div className="status" style={{backgroundColor: color}}>
      {text}
    </div>
  );
}

function Output({state, dispatch}: {state: State; dispatch: any}) {
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
      // @ts-expect-error TS2345
      setBuildState(STATUS_IDLING);
    });
  }, [state.options.numWorkers]);

  async function build() {
    // @ts-expect-error TS2345
    setBuildState(STATUS_RUNNING);

    setBuildProgress(null);

    try {
      // @ts-expect-error TS2345
      const output = await bundle(state.files, state.options, setBuildProgress);

      // @ts-expect-error TS2345
      setBuildOutput(output);
      dispatch({
        type: 'diagnostics',
        value:
          output.type === 'failure' && output.diagnostics
            ? new Map(
                [...output.diagnostics]
                  // @ts-expect-error TS2769
                  .filter(([name]: [any]) => name)
                  .map(([name, data]: [any, any]) => ['/' + name, data]),
              )
            : null,
      });
    } catch (error: any) {
      console.error('Unexpected error', error);
    }

    // @ts-expect-error TS2345
    setBuildState(STATUS_IDLING);
  }

  async function toggleWatch() {
    if (watchSubscriptionRef.current) {
      // @ts-expect-error TS2339
      watchSubscriptionRef.current.unsubscribe();
      watchSubscriptionRef.current = null;
      setWatching(false);
    } else {
      setWatching(true);
      // @ts-expect-error TS2345
      setBuildState(STATUS_RUNNING);
      let {unsubscribe, writeAssets} = await watch(
        state.files,
        state.options,
        (output) => {
          // @ts-expect-error TS2345
          setBuildState(STATUS_IDLING);
          // @ts-expect-error TS2345
          setBuildOutput(output);
          dispatch({
            type: 'diagnostics',
            value:
              output.type === 'failure' && output.diagnostics
                ? new Map(
                    [...output.diagnostics]
                      // @ts-expect-error TS2769
                      .filter(([name]: [any]) => name)
                      .map(([name, data]: [any, any]) => ['/' + name, data]),
                  )
                : null,
          });
        },
        // @ts-expect-error TS2345
        setBuildProgress,
      );
      // @ts-expect-error TS2322
      watchSubscriptionRef.current = {unsubscribe, writeAssets};
    }
  }

  useEffect(() => {
    if (watchSubscriptionRef.current) {
      // @ts-expect-error TS2339
      watchSubscriptionRef.current.writeAssets(state.files);
      // @ts-expect-error TS2345
      setBuildState(STATUS_RUNNING);
    }
  }, [state.files]);

  useKeyboard(
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
    // @ts-expect-error TS17004
    <div className="output">
      {/*
       // @ts-expect-error TS17004 */}
      <Status
        watching={watching}
        status={buildState}
        buildProgress={buildProgress}
        buildOutput={buildOutput}
      />
      {/*
       // @ts-expect-error TS17004 */}
      <div className="header">
        {/*
         // @ts-expect-error TS17004 */}
        <button
          disabled={watching || buildState !== STATUS_IDLING}
          onClick={build}
        >
          Build
        </button>
        {/*
         // @ts-expect-error TS17004 */}
        <button disabled={buildState !== STATUS_IDLING} onClick={toggleWatch}>
          {watching ? 'Stop watching' : 'Watch'}
        </button>
      </div>
      {/*
       // @ts-expect-error TS17004 */}
      <div className="files">
        {/*
         // @ts-expect-error TS2339 */}
        {buildOutput?.type === 'success' && (
          // @ts-expect-error TS17004
          <Tabs
            names={['Output', 'Preview']}
            selected={outputTabIndex}
            setSelected={setOutputTabIndex}
          >
            {/*
             // @ts-expect-error TS17004 */}
            <div>
              {/*
               // @ts-expect-error TS17004 */}
              <div className="list views">
                {/*
                 // @ts-expect-error TS2339 */}
                {buildOutput.bundles.map(({name, size, content}) => (
                  // @ts-expect-error TS17004
                  <div key={name} className="view selected">
                    {/*
                     // @ts-expect-error TS17004 */}
                    <div className="name">
                      {content.length < 500000 &&
                      // @ts-expect-error TS2339
                      buildOutput.sourcemaps?.has(name) ? (
                        // @ts-expect-error TS17004
                        <a
                          href="https://evanw.github.io/source-map-visualization/#"
                          target="_blank"
                          rel="noopener noreferrer"
                          onClick={(event) => {
                            // @ts-expect-error TS2339
                            event.target.href = linkSourceMapVisualization(
                              content,
                              // @ts-expect-error TS2339
                              nullthrows(buildOutput.sourcemaps?.get(name)),
                            );
                          }}
                        >
                          Map
                        </a>
                      ) : (
                        // @ts-expect-error TS17004
                        <span />
                      )}
                      {/*
                       // @ts-expect-error TS17004 */}
                      <span>{name}</span>
                      {/*
                       // @ts-expect-error TS17004 */}
                      <span>{filesize(size)}</span>
                    </div>
                    {/*
                     // @ts-expect-error TS17004 */}
                    <Editor name={name} value={content} readOnly />
                  </div>
                ))}
              </div>
              {/*
               // @ts-expect-error TS2339 */}
              {buildOutput?.graphs && <Graphs graphs={buildOutput.graphs} />}
            </div>
            {/*
             // @ts-expect-error TS17004 */}
            <Preview clientID={waitForFS().then(() => nullthrows(clientID))} />
          </Tabs>
        )}
        {/*
         // @ts-expect-error TS2339 */}
        {buildOutput?.type === 'failure' && (
          // @ts-expect-error TS17004
          <ParcelError output={buildOutput} />
        )}
      </div>
    </div>
  );
}

// @ts-expect-error TS7031
function Editors({state, dispatch}) {
  const views = [...state.views];
  const names = views.map(([name, data]: [any, any]) => (
    // @ts-expect-error TS17004
    <Fragment key={name}>
      {/*
       // @ts-expect-error TS17004 */}
      <span></span>
      {/*
       // @ts-expect-error TS17004 */}
      <span>{name}</span>
      {/*
       // @ts-expect-error TS17004 */}
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
      // @ts-expect-error TS17004
      return <Comp key={name} state={state} dispatch={dispatch} />;
    } else {
      return (
        // @ts-expect-error TS17004
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
      // @ts-expect-error TS17004
      <Tabs
        names={names}
        className="editors views"
        mode="hide"
        selected={state.currentView}
        // @ts-expect-error TS7006
        setSelected={(i) => dispatch({type: 'view.select', index: i})}
        // @ts-expect-error TS17004
        fallback={<Notes />}
      >
        {children}
      </Tabs>
    );
  } else {
    let merged: Array<React.ReactElement<React.ComponentProps<'div'>>> = [];
    for (let i = 0; i < views.length; i++) {
      merged.push(
        // @ts-expect-error TS17004
        <div className="view" key={i}>
          {/*
           // @ts-expect-error TS17004 */}
          <div className="name selected">{names[i]}</div>
          {/*
           // @ts-expect-error TS17004 */}
          <div className="content">{children[i]}</div>
        </div>,
      );
    }
    return (
      // @ts-expect-error TS17004
      <div className="list editors views">
        {merged}
        {/*
         // @ts-expect-error TS17004 */}
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
    // @ts-expect-error TS17004
    <FileBrowser
      files={state.files}
      collapsed={state.browserCollapsed}
      dispatch={dispatch}
      isEditing={state.isEditing}
    >
      {/*
       // @ts-expect-error TS17004 */}
      <header>
        {/*
         // @ts-expect-error TS17004 */}
        <a href="/">
          {/*
           // @ts-expect-error TS17004 */}
          <img
            className="parcel"
            src={parcelText}
            height="30"
            style={{marginTop: '5px'}}
            alt=""
          />
          {/*
           // @ts-expect-error TS17004 */}
          <img
            className="type"
            src={parcelLogo}
            style={{width: '120px'}}
            alt=""
          />
          {/*
           // @ts-expect-error TS17004 */}
          <span style={{fontSize: '25px'}}>REPL</span>
        </a>
      </header>
      {/*
       // @ts-expect-error TS17004 */}
      <div>
        {/*
         // @ts-expect-error TS17004 */}
        <PresetSelector dispatch={dispatch} />
        {/*
         // @ts-expect-error TS17004 */}
        <div className="options">
          {/*
           // @ts-expect-error TS17004 */}
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
          {/*
           // @ts-expect-error TS17004 */}
          <button
            title="Toggle view"
            className={'view ' + (state.useTabs ? 'tabs' : '')}
            onClick={() =>
              dispatch({
                type: 'toggleView',
              })
            }
          >
            {/*
             // @ts-expect-error TS17004 */}
            <span></span>
          </button>
        </div>
      </div>
    </FileBrowser>
  );

  // @ts-expect-error TS17004
  const editors = <Editors state={state} dispatch={dispatch} />;
  // @ts-expect-error TS17004
  const output = <Output state={state} dispatch={dispatch} />;

  return (
    // @ts-expect-error TS17004
    <main>
      {isDesktop ? (
        // @ts-expect-error TS17004
        <PanelGroup direction="horizontal" autoSaveId="repl-main-panels">
          {/*
           // @ts-expect-error TS17004 */}
          <Panel
            defaultSizePercentage={20}
            minSizePixels={60}
            className="panel"
          >
            {sidebar}
          </Panel>
          {/*
           // @ts-expect-error TS17004 */}
          <ResizeHandle />
          {/*
           // @ts-expect-error TS17004 */}
          <Panel
            defaultSizePercentage={45}
            minSizePixels={100}
            className="panel"
          >
            {editors}
          </Panel>
          {/*
           // @ts-expect-error TS17004 */}
          <ResizeHandle />
          {/*
           // @ts-expect-error TS17004 */}
          <Panel
            defaultSizePercentage={35}
            minSizePixels={200}
            className="panel"
          >
            {output}
          </Panel>
        </PanelGroup>
      ) : (
        // @ts-expect-error TS17004
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
  // @ts-expect-error TS17004
  return <PanelResizeHandle className="resize-handle"></PanelResizeHandle>;
}

let root = createRoot(document.getElementById('root'));
// @ts-expect-error TS17004
root.render(<App />);

if (navigator.serviceWorker) {
  navigator.serviceWorker
    // @ts-expect-error TS1470
    .register(new URL('./sw.js', import /*:: ("") */.meta.url), {
      type: 'module',
    })
    .catch((error) => {
      console.error('Service worker registration failed:', error);
    });
}
