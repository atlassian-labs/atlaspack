/* eslint-disable no-restricted-globals */
import nullthrows from 'nullthrows';

let isSafari =
  /Safari/.test(navigator.userAgent) && !/Chrome/.test(navigator.userAgent);
// @ts-expect-error - TS7034 - Variable 'lastHMRStream' implicitly has type 'any' in some locations where its type cannot be determined.
let lastHMRStream;

type ClientId = string;
type ParentId = string;

let sendToIFrame = new Map<ClientId, (data: string) => void>();
let pages = new Map<
  ParentId,
  {
    [key: string]: string;
  }
>();
let parentPorts = new Map<ParentId, MessagePort>();
let parentToIframe = new Map<ParentId, ClientId>();
let iframeToParent = new Map<ClientId, ParentId>();

// @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
global.parentPorts = parentPorts;
// @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
global.parentToIframe = parentToIframe;
// @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
global.iframeToParent = iframeToParent;

const SECURITY_HEADERS = {
  'Cross-Origin-Embedder-Policy': 'require-corp',
  'Cross-Origin-Opener-Policy': 'same-origin',
} as const;

const MIME = new Map([
  ['html', 'text/html'],
  ['js', 'text/javascript'],
  ['css', 'text/css'],
]);

// // TODO figure out which script is the entry
// function htmlWrapperForJS(script) {
//   return `<script type="application/javascript">
// window.console = {
//   log: function() {
//     var content = Array.from(arguments)
//       .map(v => (typeof v === "object" ? JSON.stringify(v) : v))
//       .join(" ");
//     document
//       .getElementById("output")
//       .appendChild(document.createTextNode(content + "\\n"));
//   },
//   warn: function() {
//     console.log.apply(console, arguments);
//   },
//   info: function() {
//     console.log.apply(console, arguments);
//   },
//   error: function() {
//     console.log.apply(console, arguments);
//   }
// };
// window.onerror = function(e) {
//   console.error(e.message);
//   console.error(e.stack);
// }
// </script>
// <body>
// Console output:<br>
// <div id="output" style="font-family: monospace;white-space: pre-wrap;"></div>
// </body>
// <script type="application/javascript">
// // try{
// ${script}
// // } catch(e){
// //   console.error(e.message);
// //   console.error(e.stack);
// // }
// </script>`;
// }

// listen here instead of attaching temporary 'message' event listeners to self
let messageProxy = new EventTarget();

self.addEventListener('message', (evt) => {
  // @ts-expect-error - TS2531 - Object is possibly 'null'. | TS2339 - Property 'id' does not exist on type 'MessageEventSource'.
  let parentId = evt.source.id;
  let {type, data, id} = evt.data;
  if (type === 'setFS') {
    // called by worker
    // @ts-expect-error - TS2531 - Object is possibly 'null'.
    evt.source.postMessage({id});
    pages.set(parentId, data);
  } else if (type === 'getID') {
    // @ts-expect-error - TS2531 - Object is possibly 'null'.
    evt.source.postMessage({id, data: parentId});
  } else if (type === 'hmrUpdate') {
    // called by worker
    // @ts-expect-error - TS2345 - Argument of type 'MessageEventSource | null' is not assignable to parameter of type 'MessagePort'.
    parentPorts.set(parentId, evt.source);
    let clientId = parentToIframe.get(parentId);
    let send =
      // @ts-expect-error - TS7005 - Variable 'lastHMRStream' implicitly has an 'any' type.
      (clientId != null ? sendToIFrame.get(clientId) : null) ?? lastHMRStream;
    send?.(data);
    // @ts-expect-error - TS2531 - Object is possibly 'null'.
    evt.source.postMessage({id});
  } else {
    let wrapper = new Event(evt.type);
    // @ts-expect-error - TS2339 - Property 'data' does not exist on type 'Event'.
    wrapper.data = evt.data;
    messageProxy.dispatchEvent(wrapper);
  }
});

let encodeUTF8 = new TextEncoder();

self.addEventListener('fetch', (evt) => {
  // @ts-expect-error - TS2339 - Property 'request' does not exist on type 'Event'.
  let url = new URL(evt.request.url);
  // @ts-expect-error - TS2339 - Property 'clientId' does not exist on type 'Event'.
  let {clientId} = evt;
  let parentId;
  if (!clientId && url.searchParams.has('parentId')) {
    // @ts-expect-error - TS2339 - Property 'resultingClientId' does not exist on type 'Event'. | TS2339 - Property 'targetClientId' does not exist on type 'Event'.
    clientId = evt.resultingClientId ?? evt.targetClientId;
    parentId = nullthrows(url.searchParams.get('parentId'));
    parentToIframe.set(parentId, clientId);
    iframeToParent.set(clientId, parentId);
  } else {
    // @ts-expect-error - TS2339 - Property 'clientId' does not exist on type 'Event'.
    parentId = iframeToParent.get(evt.clientId);
  }
  if (parentId == null && isSafari) {
    parentId = [...pages.keys()].slice(-1)[0];
  }

  if (parentId != null) {
    if (
      // @ts-expect-error - TS2339 - Property 'request' does not exist on type 'Event'.
      evt.request.headers.get('Accept') === 'text/event-stream' &&
      url.pathname === '/__parcel_hmr'
    ) {
      let stream = new ReadableStream({
        start: (controller) => {
          // @ts-expect-error - TS7006 - Parameter 'data' implicitly has an 'any' type.
          let cb = (data) => {
            let chunk = `data: ${JSON.stringify(data)}\n\n`;
            controller.enqueue(encodeUTF8.encode(chunk));
          };
          sendToIFrame.set(clientId, cb);
          lastHMRStream = cb;
        },
      });

      // @ts-expect-error - TS2339 - Property 'respondWith' does not exist on type 'Event'.
      evt.respondWith(
        new Response(stream, {
          headers: {
            'Content-Type': 'text/event-stream',
            'Transfer-Encoding': 'chunked',
            Connection: 'keep-alive',
            ...SECURITY_HEADERS,
          },
        }),
      );
    } else if (url.pathname.startsWith('/__parcel_hmr/')) {
      // @ts-expect-error - TS2339 - Property 'respondWith' does not exist on type 'Event'.
      evt.respondWith(
        (async () => {
          let port = parentId != null ? parentPorts.get(parentId) : null;

          if (port == null) {
            return new Response(null, {status: 500});
          }

          // @ts-expect-error - TS2488 - Type 'never' must have a '[Symbol.iterator]()' method that returns an iterator. | TS2554 - Expected 4 arguments, but got 3.
          let [type, content] = await sendMsg(
            port,
            'hmrAssetSource',
            url.pathname.slice('/__parcel_hmr/'.length),
          );
          return new Response(content, {
            headers: {
              'Content-Type':
                (MIME.get(type) ?? 'application/octet-stream') +
                '; charset=utf-8',
              'Cache-Control': 'no-store',
              ...SECURITY_HEADERS,
            },
          });
        })(),
      );
    } else if (url.pathname.startsWith('/__repl_dist/')) {
      let filename = url.pathname.slice('/__repl_dist/'.length);
      let file = pages.get(parentId)?.[filename];
      if (file == null) {
        console.error('requested missing file', parentId, filename, pages);
      }

      // @ts-expect-error - TS2339 - Property 'respondWith' does not exist on type 'Event'.
      evt.respondWith(
        new Response(file, {
          headers: {
            'Content-Type':
              (MIME.get(extname(filename)) ?? 'application/octet-stream') +
              '; charset=utf-8',
            'Cache-Control': 'no-store',
            ...SECURITY_HEADERS,
          },
        }),
      );
    }
  }
});

function extname(filename: string) {
  return filename.slice(filename.lastIndexOf('.') + 1);
}

// @ts-expect-error - TS7006 - Parameter 'map' implicitly has an 'any' type.
function removeNonExistingKeys(existing: Set<ClientId | ParentId>, map) {
  for (let id of map.keys()) {
    if (!existing.has(id)) {
      map.delete(id);
    }
  }
}
setInterval(async () => {
  let existingClients = new Set(
    // @ts-expect-error - TS2339 - Property 'clients' does not exist on type 'Window & typeof globalThis'. | TS7006 - Parameter 'c' implicitly has an 'any' type.
    (await self.clients.matchAll()).map((c) => c.id),
  );

  // @ts-expect-error - TS2345 - Argument of type 'Set<unknown>' is not assignable to parameter of type 'Set<string>'.
  removeNonExistingKeys(existingClients, pages);
  // @ts-expect-error - TS2345 - Argument of type 'Set<unknown>' is not assignable to parameter of type 'Set<string>'.
  removeNonExistingKeys(existingClients, sendToIFrame);
  // @ts-expect-error - TS2345 - Argument of type 'Set<unknown>' is not assignable to parameter of type 'Set<string>'.
  removeNonExistingKeys(existingClients, parentToIframe);
  // @ts-expect-error - TS2345 - Argument of type 'Set<unknown>' is not assignable to parameter of type 'Set<string>'.
  removeNonExistingKeys(existingClients, iframeToParent);
}, 20000);

function sendMsg(
  target: MessagePort,
  type: string,
  data: string,
  transfer: undefined,
) {
  let id = uuidv4();
  return new Promise((res: (result: Promise<never>) => void) => {
    let handler = (evt: MessageEvent) => {
      if (evt.data.id === id) {
        // @ts-expect-error - TS2345 - Argument of type '(evt: MessageEvent) => void' is not assignable to parameter of type 'EventListenerOrEventListenerObject | null'.
        messageProxy.removeEventListener('message', handler);
        res(evt.data.data);
      }
    };
    // @ts-expect-error - TS2345 - Argument of type '(evt: MessageEvent) => void' is not assignable to parameter of type 'EventListenerOrEventListenerObject | null'.
    messageProxy.addEventListener('message', handler);
    target.postMessage({type, data, id}, transfer);
  });
}
function uuidv4() {
  return (String(1e7) + -1e3 + -4e3 + -8e3 + -1e11).replace(
    /[018]/g,
    // $FlowFixMe
    // @ts-expect-error - TS2769 - No overload matches this call.
    (c: number) =>
      (
        c ^
        // $FlowFixMe
        (crypto.getRandomValues(new Uint8Array(1))[0] & (15 >> (c / 4)))
      ).toString(16),
  );
}
