interface TestInterface {
  test: string;
}

class Test {
  classProperty = 2;
  #privateProperty;

  constructor(text) {
    this.#privateProperty = text;
  }

  get() {
    return this.#privateProperty;
  }
}
