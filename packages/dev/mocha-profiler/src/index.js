// @flow strict-local
/**
 * This is a slightly modified version of https://github.com/Dreamscapes/mocha-profiler
 *
 * Creates a profile at `$REPOSITORY_ROOT/mocha-cpu-profiles/mocha-cpu-profile.${timestamp}.cpuprofile`
 *
 * This profile can be opened in VSCode or the Chrome Devtools performance tab.
 *
 * ## License
 *
 * Which was originally licensed with:
 *
 *     BSD 3-Clause License
 *
 *     Copyright (c) 2021, Dreamscapes
 *     All rights reserved.
 *
 *     Redistribution and use in source and binary forms, with or without
 *     modification, are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, this
 *       list of conditions and the following disclaimer.
 *
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *
 *     * Neither the name of the copyright holder nor the names of its
 *       contributors may be used to endorse or promote products derived from
 *       this software without specific prior written permission.
 *
 *     THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 *     AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 *     IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 *     DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 *     FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 *     DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 *     SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 *     CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 *     OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 *     OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * Sub-sequent changes are licensed with the LICENSE in this repository.
 */

import * as inspector from 'inspector';
import * as fs from 'fs';
import * as path from 'path';

const session = new inspector.Session();

type Done = (err?: Error) => void;

const mochaHooks = {
  beforeAll(done: Done): void {
    session.connect();

    return void session.post(
      'Profiler.enable',
      () => void session.post('Profiler.start', done),
    );
  },

  afterAll(done: Done): void {
    session.post('Profiler.stop', (sessionErr, data) => {
      if (sessionErr) {
        return void done(sessionErr);
      }

      const targetPath = path.join(
        __dirname,
        '../../../..',
        'mocha-cpu-profiles',
        `mocha-cpu-profile.${new Date().getTime()}.cpuprofile`,
      );
      fs.mkdirSync(path.dirname(targetPath), {recursive: true});

      // eslint-disable-next-line no-console
      console.log(
        `[mocha-profiler] Writing mocha CPU profile to ${targetPath}`,
      );

      return void fs.writeFile(
        targetPath,
        JSON.stringify(data.profile),
        (writeErr) => {
          if (writeErr) {
            return void done(writeErr);
          }

          session.disconnect();

          return void done();
        },
      );
    });
  },
};

export {mochaHooks};
