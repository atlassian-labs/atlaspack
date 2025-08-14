/* eslint-disable no-console */
import React from 'react';
import {a} from './a';
import {b} from './b';
import {c} from './c';
import {d} from './d';
import {e} from './e';
import {f} from './f';

export class FeatureA {
  execute(): number {
    eval('');
    const start = performance.now();
    function func1() {
      console.debug(a, b, b, c, d, e, f);
      func2();
      function func2() {
        console.debug(a, b, b, c, d, e, f);
        func3();
        function func3() {
          console.debug(a, b, b, c, d, e, f);
          func4();
          function func4() {
            console.debug(a, b, b, c, d, e, f);
            func5();
            function func5() {
              console.debug(a, b, b, c, d, e, f);
            }
          }
        }
      }
    }

    for (let i = 0; i < 10000; i++) {
      // Eval to bail out of symbol prop
      func1();
    }

    const end = performance.now();
    console.log(`FeatureA.execute execution time: ${end - start}ms`);
    return end - start;
  }
}

const feature = new FeatureA();

// Execute the feature during load
feature.execute();

export const FeatureAComponent: React.FC = () => {
  const [result, setResult] = React.useState<number>(-1);
  const [processing, setProcessing] = React.useState(false);

  const handleExecute = () => {
    setProcessing(true);

    const execResult = feature.execute();

    setResult(execResult);
    setProcessing(false);
  };

  return (
    <div style={{border: '1px solid #ccc', padding: '10px', margin: '5px'}}>
      <h4>Feature A - groupBy Test</h4>
      <p>Status: {processing ? 'Processing...' : 'Complete'}</p>
      <p>Result: {result}</p>
      <button onClick={handleExecute} disabled={processing}>
        Re-execute Feature A
      </button>
    </div>
  );
};

export {FeatureAComponent as default};
