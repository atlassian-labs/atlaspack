// flow-to-ts helpers
export type Class<T> = new (...args: any[]) => T;
// /flow-to-ts helpers

export let SharedBuffer: Class<ArrayBuffer> | Class<SharedArrayBuffer>;

// @ts-expect-error process.browser is a browser-specific property
if (process.browser) {
  SharedBuffer = ArrayBuffer;
  // Safari has removed the constructor
  if (typeof SharedArrayBuffer !== 'undefined') {
    let channel = new MessageChannel();
    try {
      // Firefox might throw when sending the Buffer over a MessagePort
      channel.port1.postMessage(new SharedArrayBuffer(0));
      SharedBuffer = SharedArrayBuffer;
    } catch (_: any) {
      // NOOP
    }
    channel.port1.close();
    channel.port2.close();
  }
} else {
  SharedBuffer = SharedArrayBuffer;
}
