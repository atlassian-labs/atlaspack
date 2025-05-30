// @flow strict-local

// https://stackoverflow.com/a/42919752
const top = 0;
const parent = (i) => ((i + 1) >>> 1) - 1;
const left = (i) => (i << 1) + 1;
const right = (i) => (i + 1) << 1;

export class PriorityQueue<T> {
  _heap: T[] = [];
  _comparator: (a: T, b: T) => number;

  constructor(comparator: (a: T, b: T) => number) {
    this._heap = [];
    this._comparator = comparator;
  }
  size(): number {
    return this._heap.length;
  }
  isEmpty(): boolean {
    return this.size() == 0;
  }
  peek(): T | void {
    return this._heap[top];
  }
  push(...values: T[]): number {
    values.forEach((value) => {
      this._heap.push(value);
      this._siftUp();
    });
    return this.size();
  }
  pop(): T | void {
    const poppedValue = this.peek();
    const bottom = this.size() - 1;
    if (bottom > top) {
      this._swap(top, bottom);
    }
    this._heap.pop();
    this._siftDown();
    return poppedValue;
  }
  _greater(i: number, j: number): number {
    return this._comparator(this._heap[i], this._heap[j]);
  }
  _swap(i: number, j: number) {
    /* $FlowIssue[unsupported-syntax] Flow thinks that parameters are consts */
    [this._heap[i], this._heap[j]] = [this._heap[j], this._heap[i]];
  }
  _siftUp() {
    let node = this.size() - 1;
    while (node > top && this._greater(node, parent(node))) {
      this._swap(node, parent(node));
      node = parent(node);
    }
  }
  _siftDown() {
    let node = top;
    while (
      (left(node) < this.size() && this._greater(left(node), node)) ||
      (right(node) < this.size() && this._greater(right(node), node))
    ) {
      let maxChild =
        right(node) < this.size() && this._greater(right(node), left(node))
          ? right(node)
          : left(node);
      this._swap(node, maxChild);
      node = maxChild;
    }
  }
}
