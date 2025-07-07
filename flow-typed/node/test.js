// Missing flow types for node:test
declare module "node:test" {
  declare module.exports: {
    describe(message?: string, callback: any): void,
    test(message?: string, callback: any): void,
    it(message?: string, callback: any): void,
    before(callback: any): void,
    beforeEach(callback: any): void,
    after(callback: any): void,
    afterEach(callback: any): void,
    ...
  }
}

