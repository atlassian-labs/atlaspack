import {sendAnalyticsEvent} from '../cmd/index.js';
import {mock} from 'node:test';
import type {Mock} from 'node:test';
import assert from 'node:assert';

describe('@atlaspack/analytics', () => {
  let dispatchEventMock: Mock<Window['dispatchEvent']>;

  beforeEach(() => {
    // @ts-expect-error
    globalThis.CustomEvent = MockCustomEvent;
    dispatchEventMock = mock.fn();
    globalThis.dispatchEvent = dispatchEventMock;
  });

  it('should not throw', () => {
    assert.doesNotThrow(() =>
      sendAnalyticsEvent({
        action: 'test',
      }),
    );
  });

  it('should raise event on window', () => {
    sendAnalyticsEvent({
      action: 'test',
    });

    assert.equal(dispatchEventMock.mock.callCount(), 1);
    assert.deepEqual(dispatchEventMock.mock.calls[0].arguments[0], {
      eventName: 'atlaspack:analytics',
      detail: {
        action: 'test',
      },
    });
  });
});

class MockCustomEvent {
  eventName: string;
  detail: any;

  constructor(eventName: string, options: any = {}) {
    this.eventName = eventName;
    this.detail = options.detail;
  }
}
