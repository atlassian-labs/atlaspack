import path from 'path';
// @ts-expect-error TS6142
import {EditableField} from './helper';
import {downloadZIP} from '../utils';

// @ts-expect-error TS7006
function join(a, ...b) {
  return path.join(a || '/', ...b);
}

function FileBrowserEntry({
  // @ts-expect-error TS7031
  name,
  // @ts-expect-error TS7031
  prefix,
  directory = false,
  // @ts-expect-error TS7031
  isEntry,
  // @ts-expect-error TS7031
  isEditing,
  // @ts-expect-error TS7031
  collapsed,
  // @ts-expect-error TS7031
  children,
  // @ts-expect-error TS7031
  dispatch,
  ...rest
}) {
  let p = join(prefix, name);
  return (
    // @ts-expect-error TS17004
    <li
      draggable="true"
      onDragStart={(e) => {
        // @ts-expect-error TS2339
        e.dataTransfer.setData('application/x-parcel-repl-file', p);
        e.stopPropagation();
      }}
      {...rest}
    >
      {/*
       // @ts-expect-error TS17004 */}
      <div
        className={directory ? `dir ${!collapsed ? 'expanded' : ''}` : 'file'}
        onClick={() =>
          directory
            ? dispatch({
                type: 'browser.expandToggle',
                name: p,
              })
            : dispatch({
                type: 'view.open',
                name: p,
              })
        }
        // tabIndex="0"
        // onDblclick={(e) => console.log(e)}
      >
        {/*
         // @ts-expect-error TS17004 */}
        <div className="name">
          {/*
           // @ts-expect-error TS17004 */}
          <div className="icon" />
          {/*
           // @ts-expect-error TS17004 */}
          <EditableField
            value={name}
            editing={isEditing === p}
            // @ts-expect-error TS7006
            onChange={(value) =>
              dispatch({
                type: 'browser.setEditing',
                value,
              })
            }
          />
        </div>
        {/*
         // @ts-expect-error TS17004 */}
        <div className="controls">
          {!directory && (
            // @ts-expect-error TS17004
            <input
              title="Entrypoint"
              type="checkbox"
              checked={isEntry}
              onChange={(e) => {
                dispatch({
                  type: 'file.isEntry',
                  name: p,
                  // @ts-expect-error TS2339
                  value: e.target.checked,
                });
                e.stopPropagation();
              }}
            />
          )}
          {/*
           // @ts-expect-error TS17004 */}
          <button
            className="rename"
            onClick={(e) => {
              dispatch({
                type: 'browser.setEditing',
                name: p,
              });
              e.stopPropagation();
            }}
          />
          {/*
           // @ts-expect-error TS17004 */}
          <button
            className="delete"
            onClick={(e) => {
              dispatch({
                type: 'file.delete',
                name: p,
              });
              e.stopPropagation();
            }}
          />
        </div>
      </div>
      {children}
    </li>
  );
}

function FileBrowserFolder({
  // @ts-expect-error TS7031
  files,
  // @ts-expect-error TS7031
  collapsed,
  // @ts-expect-error TS7031
  dispatch,
  // @ts-expect-error TS7031
  isEditing,
  prefix = '',
  ...props
}) {
  return (
    // @ts-expect-error TS17004
    <ul {...props}>
      {[...files]
        .sort(([a]: [any], [b]: [any]) => a.localeCompare(b))
        .map(([name, data]: [any, any]) => {
          let p = join(prefix, name);
          let isCollapsed = collapsed.has(p);
          return data instanceof Map ? (
            // @ts-expect-error TS17004
            <FileBrowserEntry
              key={name}
              directory
              name={name}
              prefix={prefix}
              dispatch={dispatch}
              isEditing={isEditing}
              collapsed={isCollapsed}
              // @ts-expect-error TS7006
              onDrop={(e) => {
                const data = e.dataTransfer.getData(
                  'application/x-parcel-repl-file',
                );
                if (data !== p) {
                  dispatch({
                    type: 'file.move',
                    name: data,
                    dir: p,
                  });
                  e.preventDefault();
                  e.stopPropagation();
                }
              }}
              // @ts-expect-error TS7006
              onDragOver={(e) => e.preventDefault()}
            >
              {!isCollapsed && (
                // @ts-expect-error TS17004
                <FileBrowserFolder
                  files={data}
                  collapsed={collapsed}
                  dispatch={dispatch}
                  isEditing={isEditing}
                  prefix={p}
                />
              )}
            </FileBrowserEntry>
          ) : (
            // @ts-expect-error TS17004
            <FileBrowserEntry
              key={name}
              name={name}
              isEntry={!!data.isEntry}
              prefix={prefix}
              dispatch={dispatch}
              isEditing={isEditing}
            />
          );
        })}
    </ul>
  );
}

export function FileBrowser({
  files,
  collapsed,
  isEditing,
  dispatch,
  children,
}: any): any {
  return (
    // @ts-expect-error TS17004
    <div className="file-browser">
      {children}
      {/*
       // @ts-expect-error TS17004 */}
      <div>
        {/*
         // @ts-expect-error TS17004 */}
        <div className="header">
          {/*<button
          onClick={async () => {
            const dirHandle = await window.showDirectoryPicker();
            for await (const entry of dirHandle.values()) {
              console.log(entry.kind, entry.name);
            }
          }}
        >
          Open
        </button>*/}
          {/*
           // @ts-expect-error TS17004 */}
          <button onClick={() => dispatch({type: 'file.addFolder'})}>
            New Folder
          </button>
          {/*
           // @ts-expect-error TS17004 */}
          <button onClick={() => dispatch({type: 'file.addFile'})}>
            New File
          </button>
        </div>
        {/*
         // @ts-expect-error TS17004 */}
        <FileBrowserFolder
          files={files}
          collapsed={collapsed}
          isEditing={isEditing}
          dispatch={dispatch}
          // @ts-expect-error TS7006
          onDrop={(e) => {
            const data = e.dataTransfer.getData(
              'application/x-parcel-repl-file',
            );
            dispatch({type: 'file.move', name: data, dir: ''});
            e.preventDefault();
            e.stopPropagation();
          }}
          // @ts-expect-error TS7006
          onDragOver={(e) => e.preventDefault()}
        />
        {/*
         // @ts-expect-error TS17004 */}
        <div className="download">
          {/*
           // @ts-expect-error TS17004 */}
          <button
            onClick={() => {
              downloadZIP(files.list());
            }}
          >
            Download
          </button>
        </div>
      </div>
    </div>
  );
}
