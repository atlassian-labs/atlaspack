import type { Session } from "inspector";
import ThrowableDiagnostic from "@atlaspack/diagnostic";

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
  session: Session | undefined;

  async startProfiling(): Promise<unknown> {
    let inspector: typeof import('inspector');

    try {
      inspector = await import('inspector');
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    } catch (err) {
      throw new ThrowableDiagnostic({
        diagnostic: {
          message: `The inspector module isn't available`,
          origin: '@atlaspack/workers',
          hints: ['Disable build profiling']
        }
      });
    }
    if (!this.session) {
      this.session = new inspector.Session()
    }
    this.session.connect();
    return Promise.all([this.sendCommand('Profiler.setSamplingInterval', {
      interval: 100
    }), this.sendCommand('Profiler.enable'), this.sendCommand('Profiler.start')]);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  sendCommand(method: string, params?: any): Promise<{
    profile: Profile;
  }> {
    if (!this.session) {
      throw new Error('No session set')
    }
    return new Promise((resolve, reject) => {
      this.session!.post(method, params, (err, p) => {
        if (err == null) {
          resolve((p as {
            profile: Profile;
          }));
        } else {
          reject(err);
        }
      });
    });
  }

  destroy() {
    if (this.session != null) {
      this.session.disconnect();
    }
  }

  async stopProfiling(): Promise<Profile> {
    const res = await this.sendCommand('Profiler.stop');
    this.destroy();
    return res.profile;
  }
}
