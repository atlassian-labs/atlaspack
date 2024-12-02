interface TestInterface {
  test: string;
}

class Test {
  #privateProperty: string;

  classProperty = 2;

  constructor(text) {
    this.#privateProperty = text;
  }

  getProperty() {
    return this.#privateProperty;
  }
}

import('./main');
