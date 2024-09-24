await $`rm -f data.json`;
await $`wget https://raw.githubusercontent.com/Fyrd/caniuse/main/data.json`;
const fs = require("fs");
const data = require("./data.json");

const browserAgents = [];
Object.entries(data.agents).forEach(([key, agent]) => {
  browserAgents.push({
    name: capitalize(key),
    comment: [...agent.long_name.split("\n")],
    key,
  });
});
console.log("use serde::{Serialize, Deserialize};\n");
console.log(
  "#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]"
);
console.log("pub enum BrowserAgent {");
browserAgents.forEach((agent) => {
  agent.comment.forEach((comment) => {
    console.log(`    /// ${comment}`);
  });
  console.log(`    ${agent.name},`);
});
console.log(`    /// Any other browser`);
console.log(`    Any(String),`);
console.log("}");
console.log("impl BrowserAgent {");
console.log("    pub fn key(&self) -> &str {");
console.log("        match self {");
browserAgents.forEach((agent) => {
  console.log(`          BrowserAgent::${agent.name} => "${agent.key}",`);
});
console.log(`          BrowserAgent::Any(key) => key,`);
console.log("        }");
console.log("    }");
console.log("    pub fn from_key(key: &str) -> Self {");
console.log("        match key {");
browserAgents.forEach((agent) => {
  console.log(`          "${agent.key}" => BrowserAgent::${agent.name},`);
});
console.log("            key => BrowserAgent::Any(key.to_string()),");
console.log("        }");
console.log("    }");
console.log("}");

console.log("");

const featuresEnum = [];
Object.entries(data.data).forEach(([key, feature]) => {
  // console.log(feature.stats);
  featuresEnum.push({
    name: capitalize(key),
    key,
    comment: [
      ...feature.title.split("\n"),
      "",
      ...feature.description.split("\n"),
      "",
      ...feature.links.map((link) => `* [${link.title}](${link.url})`),
    ],
  });
});

console.log(
  "#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]"
);
console.log("pub enum BrowserFeature {");
featuresEnum.forEach((feature) => {
  feature.comment.forEach((comment) => {
    console.log(`    /// ${comment}`);
  });
  console.log(`    ${feature.name},`);
});
console.log(`    /// Any other browser feature`);
console.log(`    Any(String),`);
console.log("}");

console.log("impl BrowserFeature {");
console.log("    pub fn key(&self) -> &str {");
console.log("        match self {");
featuresEnum.forEach((feature) => {
  console.log(`          BrowserFeature::${feature.name} => "${feature.key}",`);
});
console.log(`          BrowserFeature::Any(key) => key,`);
console.log("        }");
console.log("    }");
console.log("    pub fn from_key(key: &str) -> Self {");
console.log("        match key {");
featuresEnum.forEach((feature) => {
  console.log(`          "${feature.key}" => BrowserFeature::${feature.name},`);
});
console.log("            key => BrowserFeature::Any(key.to_string()),");
console.log("        }");
console.log("    }");
console.log("}");

const minimalData = {};
Object.entries(data.data).forEach(([key, feature]) => {
  const minimalStats = {};
  for (let [browser, versions] of Object.entries(feature.stats)) {
    const minimalVersions = {};
    const requirements = collapseRequirements(versions);
    for (let range of requirements) {
      minimalVersions[range] = 1;
    }
    minimalStats[browser] = minimalVersions;
  }
  minimalData[key] = minimalStats;
});

fs.writeFileSync("src/data.json", JSON.stringify(minimalData, null, 2));

function capitalize(s) {
  const parts = s.split(/[\W_]/g);
  return parts
    .map((part) => {
      return part.charAt(0).toUpperCase() + part.slice(1);
    })
    .join("");
}

function parseVersion(s) {
  const parts = s.split(".");
  return parts.map((part) => parseInt(part));
}

function parseVersionRange(s) {
  const parts = s.split("-");
  return parts.map((part) => parseVersion(part));
}

function collapseRequirements(versions) {
  const ranges = [];
  let currentMinimum = null;
  let currentMaximum = null;

  for (let [version, supports] of Object.entries(versions)) {
    if (supports === "y") {
      if (currentMinimum === null) {
        currentMinimum = version;
      }
      currentMaximum = version;
    } else {
      if (currentMinimum !== null) {
        const range = [currentMinimum];
        if (currentMaximum !== currentMinimum) {
          range.push(currentMaximum);
        }
        ranges.push(range.join("-"));
      }
      currentMinimum = null;
      currentMaximum = null;
    }
  }
  if (currentMinimum !== null) {
    const range = [currentMinimum];
    if (currentMaximum !== currentMinimum) {
      range.push(currentMaximum);
    }
    ranges.push(range.join("-"));
  }
  return ranges;
}
