import path from 'path';
// @ts-expect-error - TS6142 - Module './helper' was resolved to '/home/ubuntu/parcel/packages/dev/repl/src/components/helper.tsx', but '--jsx' is not set.
import {EditableField} from './helper';
import {downloadZIP} from '../utils';

// @ts-expect-error - TS7006 - Parameter 'a' implicitly has an 'any' type. | TS7019 - Rest parameter 'b' implicitly has an 'any[]' type.
function join(a, ...b) {
  return path.join(a || '/', ...b);
}

function FileBrowserEntry({
// @ts-expect-error - TS7031 - Binding element 'name' implicitly has an 'any' type.
  name,
// @ts-expect-error - TS7031 - Binding element 'prefix' implicitly has an 'any' type.
  prefix,
  directory = false,
// @ts-expect-error - TS7031 - Binding element 'isEntry' implicitly has an 'any' type.
  isEntry,
// @ts-expect-error - TS7031 - Binding element 'isEditing' implicitly has an 'any' type.
  isEditing,
// @ts-expect-error - TS7031 - Binding element 'collapsed' implicitly has an 'any' type.
  collapsed,
// @ts-expect-error - TS7031 - Binding element 'children' implicitly has an 'any' type.
  children,
// @ts-expect-error - TS7031 - Binding element 'dispatch' implicitly has an 'any' type.
  dispatch,
  ...rest
}) {
  let p = join(prefix, name);
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <li
      draggable="true"
      onDragStart={(e) => {
        e.dataTransfer.setData('application/x-parcel-repl-file', p);
        e.stopPropagation();
      }}
      {...rest}
    >
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
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
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <div className="name">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <div className="icon" />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <EditableField
            value={name}
            editing={isEditing === p}
// @ts-expect-error - TS7006 - Parameter 'value' implicitly has an 'any' type.
            onChange={(value) =>
              dispatch({
                type: 'browser.setEditing',
                value,
              })
            }
          />
        </div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <div className="controls">
          {!directory && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <input
              title="Entrypoint"
              type="checkbox"
              checked={isEntry}
              onChange={(e) => {
                dispatch({
                  type: 'file.isEntry',
                  name: p,
                  value: e.target.checked,
                });
                e.stopPropagation();
              }}
            />
          )}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
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
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
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
// @ts-expect-error - TS7031 - Binding element 'files' implicitly has an 'any' type.
  files,
// @ts-expect-error - TS7031 - Binding element 'collapsed' implicitly has an 'any' type.
  collapsed,
// @ts-expect-error - TS7031 - Binding element 'dispatch' implicitly has an 'any' type.
  dispatch,
// @ts-expect-error - TS7031 - Binding element 'isEditing' implicitly has an 'any' type.
  isEditing,
  prefix = '',
  ...props
}) {
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <ul {...props}>
      {[...files]
        .sort(([a]: [any], [b]: [any]) => a.localeCompare(b))
        .map(([name, data]: [any, any]) => {
          let p = join(prefix, name);
          let isCollapsed = collapsed.has(p);
          return data instanceof Map ? (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. | TS2741 - Property 'isEntry' is missing in type '{ children: false | Element; key: any; directory: true; name: any; prefix: string; dispatch: any; isEditing: any; collapsed: any; onDrop: (e: any) => void; onDragOver: (e: any) => any; }' but required in type '{ [x: string]: any; name: any; prefix: any; directory?: boolean | undefined; isEntry: any; isEditing: any; collapsed: any; children: any; dispatch: any; }'.
            <FileBrowserEntry
              key={name}
              directory
              name={name}
              prefix={prefix}
              dispatch={dispatch}
              isEditing={isEditing}
              collapsed={isCollapsed}
// @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
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
// @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
              onDragOver={(e) => e.preventDefault()}
            >
              {!isCollapsed && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
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
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. | TS2739 - Type '{ key: any; name: any; isEntry: boolean; prefix: string; dispatch: any; isEditing: any; }' is missing the following properties from type '{ [x: string]: any; name: any; prefix: any; directory?: boolean | undefined; isEntry: any; isEditing: any; collapsed: any; children: any; dispatch: any; }': collapsed, children
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

export function FileBrowser(
  {
    files,
    collapsed,
    isEditing,
    dispatch,
    children,
  }: any,
): any {
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div className="file-browser">
      {children}
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
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
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <button onClick={() => dispatch({type: 'file.addFolder'})}>
            New Folder
          </button>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
          <button onClick={() => dispatch({type: 'file.addFile'})}>
            New File
          </button>
        </div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <FileBrowserFolder
          files={files}
          collapsed={collapsed}
          isEditing={isEditing}
          dispatch={dispatch}
// @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
          onDrop={(e) => {
            const data = e.dataTransfer.getData(
              'application/x-parcel-repl-file',
            );
            dispatch({type: 'file.move', name: data, dir: ''});
            e.preventDefault();
            e.stopPropagation();
          }}
// @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
          onDragOver={(e) => e.preventDefault()}
        />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <div className="download">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
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
