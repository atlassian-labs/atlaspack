// @ts-expect-error TS2307
import { Person } from "original";
Person.prototype.greet = function() { return `Hello ${this.name}!` }

export const anotherThing: string = "hello";

// @ts-expect-error TS2664
declare module "original" {
  interface Person {
    greet(): string;
  }
}

export const somethingElse: string = "goodbye";