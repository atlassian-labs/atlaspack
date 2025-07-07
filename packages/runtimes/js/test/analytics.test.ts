import {sendAnalyticsEvent} from '../src/helpers/browser/analytics/analytics.js';
import {mock} from 'node:test';

import type {Mock} from 'node:test';
import {describe, it, beforeEach} from 'node:test';
import assert from 'node:assert';

describe('@atlaspack/analytics', () => {
  let dispatchEventMock: Mock<Window['dispatchEvent']>;

  beforeEach(() => {
    // @ts-expect-error - Mocking CustomEvent for testing
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  detail: any;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  constructor(eventName: string, options: any = {}) {
    this.eventName = eventName;
    this.detail = options.detail;
  }
}
