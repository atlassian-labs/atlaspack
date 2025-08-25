const Benchmark = require("benchmark");
const suite = new Benchmark.Suite();
const path = require("path");

suite
  .add("run bundled app", () => {
    require(path.join(__dirname, "dist/bundle.js"));
  })
  .on("cycle", (event) => {
    console.log(String(event.target));
  })
  .on("complete", function () {
    console.log("Fastest is " + this.filter("fastest").map("name"));
  })
  .run({ async: true });
