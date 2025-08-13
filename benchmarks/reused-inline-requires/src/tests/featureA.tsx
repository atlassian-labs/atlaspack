import React from 'react';
import assign from 'lodash/assign';
import cloneDeep from 'lodash/cloneDeep';
import debounce from 'lodash/debounce';
import difference from 'lodash/difference';
import filter from 'lodash/filter';
import find from 'lodash/find';
import flatten from 'lodash/flatten';
import forEach from 'lodash/forEach';
import get from 'lodash/get';
import groupBy from 'lodash/groupBy';
import has from 'lodash/has';
import intersection from 'lodash/intersection';
import isEqual from 'lodash/isEqual';
import map from 'lodash/map';
import merge from 'lodash/merge';
import omit from 'lodash/omit';
import pick from 'lodash/pick';
import reduce from 'lodash/reduce';
import set from 'lodash/set';
import sortBy from 'lodash/sortBy';
import throttle from 'lodash/throttle';
import uniq from 'lodash/uniq';

export class FeatureA {
  execute(): number {
    const start = performance.now();
    function func1() {
      console.debug(
        assign,
        cloneDeep,
        debounce,
        difference,
        filter,
        find,
        flatten,
        forEach,
        get,
        groupBy,
        has,
        intersection,
        isEqual,
        map,
        merge,
        omit,
        pick,
        reduce,
        set,
        sortBy,
        throttle,
        uniq,
      );
      func2();
      function func2() {
        console.debug(
          assign,
          cloneDeep,
          debounce,
          difference,
          filter,
          find,
          flatten,
          forEach,
          get,
          groupBy,
          has,
          intersection,
          isEqual,
          map,
          merge,
          omit,
          pick,
          reduce,
          set,
          sortBy,
          throttle,
          uniq,
        );
        func3();
        function func3() {
          console.debug(
            assign,
            cloneDeep,
            debounce,
            difference,
            filter,
            find,
            flatten,
            forEach,
            get,
            groupBy,
            has,
            intersection,
            isEqual,
            map,
            merge,
            omit,
            pick,
            reduce,
            set,
            sortBy,
            throttle,
            uniq,
          );
          func4();
          function func4() {
            console.debug(
              assign,
              cloneDeep,
              debounce,
              difference,
              filter,
              find,
              flatten,
              forEach,
              get,
              groupBy,
              has,
              intersection,
              isEqual,
              map,
              merge,
              omit,
              pick,
              reduce,
              set,
              sortBy,
              throttle,
              uniq,
            );
            func5();
            function func5() {
              console.debug(
                assign,
                cloneDeep,
                debounce,
                difference,
                filter,
                find,
                flatten,
                forEach,
                get,
                groupBy,
                has,
                intersection,
                isEqual,
                map,
                merge,
                omit,
                pick,
                reduce,
                set,
                sortBy,
                throttle,
                uniq,
              );
              func6();
              function func6() {
                console.debug(
                  assign,
                  cloneDeep,
                  debounce,
                  difference,
                  filter,
                  find,
                  flatten,
                  forEach,
                  get,
                  groupBy,
                  has,
                  intersection,
                  isEqual,
                  map,
                  merge,
                  omit,
                  pick,
                  reduce,
                  set,
                  sortBy,
                  throttle,
                  uniq,
                );
                func7();
                function func7() {
                  console.debug(
                    assign,
                    cloneDeep,
                    debounce,
                    difference,
                    filter,
                    find,
                    flatten,
                    forEach,
                    get,
                    groupBy,
                    has,
                    intersection,
                    isEqual,
                    map,
                    merge,
                    omit,
                    pick,
                    reduce,
                    set,
                    sortBy,
                    throttle,
                    uniq,
                  );
                  func8();
                  function func8() {
                    console.debug(
                      assign,
                      cloneDeep,
                      debounce,
                      difference,
                      filter,
                      find,
                      flatten,
                      forEach,
                      get,
                      groupBy,
                      has,
                      intersection,
                      isEqual,
                      map,
                      merge,
                      omit,
                      pick,
                      reduce,
                      set,
                      sortBy,
                      throttle,
                      uniq,
                    );
                    func9();
                    function func9() {
                      console.debug(
                        assign,
                        cloneDeep,
                        debounce,
                        difference,
                        filter,
                        find,
                        flatten,
                        forEach,
                        get,
                        groupBy,
                        has,
                        intersection,
                        isEqual,
                        map,
                        merge,
                        omit,
                        pick,
                        reduce,
                        set,
                        sortBy,
                        throttle,
                        uniq,
                      );
                    }
                  }
                }
              }
            }
          }
        }
      }
    }

    for (let i = 0; i < 10000; i++) {
      func1();
    }

    const end = performance.now();
    console.log(`FeatureA.execute execution time: ${end - start}ms`);
    return end - start;
  }
}

const feature = new FeatureA();

// Execute the feature during load
feature.execute()

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
