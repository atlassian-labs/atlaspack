import {Flow} from 'flow-to-typescript-codemod';
// Copy from @atlaspack/utils to fix: https://github.com/stackblitz/core/issues/1855
export let SharedBuffer:
  | Flow.Class<ArrayBuffer>
  | Flow.Class<SharedArrayBuffer>;

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
