import {useRef, useState} from 'react';
// @ts-expect-error TS6142
import {usePromise} from './helper';

export function Preview({clientID}: {clientID: Promise<string>}): any {
  let [clientIDResolved] = usePromise(clientID);
  let url =
    clientIDResolved && `/__repl_dist/index.html?parentId=${clientIDResolved}`;
  let [popover, setPopover] = useState(null);

  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  // TODO disable preview if options.publicURL !== '/__repl_dist'

  return (
    url && (
      // @ts-expect-error TS17004
      <div className="preview">
        {/*
         // @ts-expect-error TS17004 */}
        <div className="controls">
          {!popover && (
            // @ts-expect-error TS17004
            <button
              onClick={() => {
                // @ts-expect-error TS2304
                let w = window.open(url);
                // window.open(url, '_blank', 'toolbar=0,location=0,menubar=0'),
                setPopover(w);
                w.onload = function () {
                  this.onbeforeunload = function () {
                    setPopover(null);
                  };
                };
              }}
              disabled={!url}
            >
              Move to new window
            </button>
          )}
          {popover && (
            // @ts-expect-error TS17004
            <button
              onClick={() => {
                // @ts-expect-error TS2339
                popover.close();
                setPopover(null);
              }}
              disabled={!url}
            >
              Close popover
            </button>
          )}
          {!popover && (
            // @ts-expect-error TS17004
            <button
              className="reload"
              // @ts-expect-error TS18047
              onClick={() => (iframeRef.current.src = url)}
            >
              Reload
            </button>
          )}
        </div>
        {!popover && (
          //<Box>
          // @ts-expect-error TS17004
          <iframe title="Preview" ref={iframeRef} src={url} />
          //</Box>
        )}
      </div>
    )
  );
}
