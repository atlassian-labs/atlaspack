import {useRef, useState} from 'react';
// @ts-expect-error - TS6142 - Module './helper' was resolved to '/home/ubuntu/parcel/packages/dev/repl/src/components/helper.tsx', but '--jsx' is not set.
import {usePromise} from './helper';

export function Preview(
  {
    clientID,
  }: {
    clientID: Promise<string>
  },
): any {
  let [clientIDResolved] = usePromise(clientID);
  let url =
    clientIDResolved && `/__repl_dist/index.html?parentId=${clientIDResolved}`;
  let [popover, setPopover] = useState(null);

  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  // TODO disable preview if options.publicURL !== '/__repl_dist'

  return (
    url && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
      <div className="preview">
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
        <div className="controls">
          {!popover && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <button
              onClick={() => {
                let w = window.open(url);
                // window.open(url, '_blank', 'toolbar=0,location=0,menubar=0'),
// @ts-expect-error - TS2345 - Argument of type 'Window | null' is not assignable to parameter of type 'SetStateAction<null>'.
                setPopover(w);
// @ts-expect-error - TS2531 - Object is possibly 'null'.
                w.onload = function () {
// @ts-expect-error - TS2339 - Property 'onbeforeunload' does not exist on type 'GlobalEventHandlers'.
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
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <button
              onClick={() => {
// @ts-expect-error - TS2531 - Object is possibly 'null'.
                popover.close();
                setPopover(null);
              }}
              disabled={!url}
            >
              Close popover
            </button>
          )}
          {!popover && (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
            <button
              className="reload"
              // $FlowFixMe
// @ts-expect-error - TS2531 - Object is possibly 'null'.
              onClick={() => (iframeRef.current.src = url)}
            >
              Reload
            </button>
          )}
        </div>
        {!popover && (
          //<Box>
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
          <iframe title="Preview" ref={iframeRef} src={url} />
          //</Box>
        )}
      </div>
    )
  );
}
