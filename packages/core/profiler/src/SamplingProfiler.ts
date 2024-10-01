import type {Session} from 'inspector';
import invariant from 'assert';
import ThrowableDiagnostic from '@atlaspack/diagnostic';

// https://chromedevtools.github.io/devtools-protocol/tot/Profiler#type-Profile
export type Profile = {
  nodes: Array<ProfileNode>;
  startTime: number;
  endTime: number;
  samples?: Array<number>;
  timeDeltas?: Array<number>;
};

// https://chromedevtools.github.io/devtools-protocol/tot/Profiler#type-ProfileNode
type ProfileNode = {
  id: number;
  callFrame: CallFrame;
  hitCount?: number;
  children?: Array<number>;
  deoptReason?: string;
  positionTicks?: PositionTickInfo;
};

// https://chromedevtools.github.io/devtools-protocol/tot/Runtime#type-CallFrame
type CallFrame = {
  functionName: string;
  scriptId: string;
  url: string;
  lineNumber: string;
  columnNumber: string;
};

// https://chromedevtools.github.io/devtools-protocol/tot/Profiler#type-PositionTickInfo
type PositionTickInfo = {
  line: number;
  ticks: number;
};

export default class SamplingProfiler {
  // @ts-expect-error - TS2564 - Property 'session' has no initializer and is not definitely assigned in the constructor.
  session: Session;

  startProfiling(): Promise<unknown> {
    let inspector;
    try {
      inspector = require('inspector');
    } catch (err: any) {
      throw new ThrowableDiagnostic({
        diagnostic: {
          message: `The inspector module isn't available`,
          origin: '@atlaspack/workers',
          hints: ['Disable build profiling'],
        },
      });
    }

    this.session = new inspector.Session();
    this.session.connect();

    return Promise.all([
      this.sendCommand('Profiler.setSamplingInterval', {
        interval: 100,
      }),
      this.sendCommand('Profiler.enable'),
      this.sendCommand('Profiler.start'),
    ]);
  }

  sendCommand(
    method: string,
    params?: unknown,
  ): Promise<{
    profile: Profile;
  }> {
    invariant(this.session != null);
    return new Promise(
      (
        resolve: (
          result:
            | Promise<{
                profile: Profile;
              }>
            | {
                profile: Profile;
              },
        ) => void,
        reject: (error?: any) => void,
      ) => {
        // @ts-expect-error - TS2769 - No overload matches this call. | TS7006 - Parameter 'err' implicitly has an 'any' type. | TS7006 - Parameter 'p' implicitly has an 'any' type.
        this.session.post(method, params, (err, p) => {
          if (err == null) {
            resolve(
              p as {
                profile: Profile;
              },
            );
          } else {
            reject(err);
          }
        });
      },
    );
  }

  destroy() {
    if (this.session != null) {
      this.session.disconnect();
    }
  }

  async stopProfiling(): Promise<Profile> {
    let res = await this.sendCommand('Profiler.stop');
    this.destroy();
    return res.profile;
  }
}
