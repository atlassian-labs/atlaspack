import {$} from 'zx';
import fs from 'node:fs/promises';

// Artifacts directory (declared early so resume can read results before mkdir)
const artifactsDir = 'benchmark-artifacts';
const MAX_FILES = 100000;

async function readJsonIfExists(path) {
  try {
    const data = await fs.readFile(path, 'utf8');
    return JSON.parse(data);
  } catch {
    return null;
  }
}

function hasEntryByName(arr, name) {
  return Array.isArray(arr) && arr.some((d) => d && d.name === name);
}

function getEntryByName(arr, name) {
  if (!Array.isArray(arr)) return null;
  return arr.find((d) => d && d.name === name) || null;
}

async function loadPreviousResults() {
  const prev = await readJsonIfExists(`${artifactsDir}/results.json`);
  if (!prev || typeof prev !== 'object') {
    return {files: [], depth: [], subtrees: [], lines: [], asyncRatio: []};
  }
  return {
    files: Array.isArray(prev.files) ? prev.files : [],
    depth: Array.isArray(prev.depth) ? prev.depth : [],
    subtrees: Array.isArray(prev.subtrees) ? prev.subtrees : [],
    lines: Array.isArray(prev.lines) ? prev.lines : [],
    asyncRatio: Array.isArray(prev.asyncRatio) ? prev.asyncRatio : [],
  };
}

$.verbose = true;

async function generate({
  files = 100,
  avgLinesPerFile = 10,
  depth,
  subtrees,
  avgOutDegree = 1.1,
  asyncImportRatio,
  generateDot = true,
  name = 'benchmark',
} = {}) {
  await $`rm -rf ${name}`;

  const args = [
    'generate-project',
    '--subtrees',
    subtrees ?? Math.ceil(files / 10),
    '--cross-edge-prob',
    0.03,
    '--files',
    files,
    '--avg-lines-per-file',
    avgLinesPerFile,
    '--depth',
    depth ?? Math.ceil(files / 2),
    '--avg-out-degree',
    avgOutDegree,
    ...(asyncImportRatio != null ? ['--async-import-ratio', asyncImportRatio] : []),
    name,
    ...((generateDot && files < 10000) ? ['--dot-output', `${name}/graph.png`] : []),
  ];
  await $`cargo run --release -- ${args}`;
}

async function runBenchmark({
  name = 'example',
  files,
  avgLinesPerFile,
  depth,
  subtrees,
  avgOutDegree,
  asyncImportRatio,
  generateDot = true,
}) {
await generate({
    files,
    avgLinesPerFile,
    depth,
    subtrees,
    avgOutDegree,
    asyncImportRatio,
    generateDot,
    name,
  });

  const start = Date.now();
  try {
    await $`yarn atlaspack build --no-cache $PWD/${name}/app-root/src/index.ts`;
    const end = Date.now();
    return {time: end - start, crashed: false};
  } catch (_err) {
    const end = Date.now();
    return {time: end - start, crashed: true};
  }
}

const SAMPLE_COUNT = 3;
const TARGET_10S = 10_000;
const TOL_FRAC = 0.2; // 20% tolerance

async function measurePoint(
  {
    baseName,
    files,
    avgLinesPerFile,
    depth,
    subtrees,
    avgOutDegree,
    asyncImportRatio,
  },
  copyGraphDir,
) {
  const times = [];
  let crashedCount = 0;
  let graph = null;
  let graphExists = false;
  const generateDot = Boolean(copyGraphDir);
  const sampleCount = copyGraphDir ? SAMPLE_COUNT : 1;
  for (let i = 0; i < sampleCount; i++) {
    const name = `${baseName}-s${i + 1}`;
    const {time, crashed} = await runBenchmark({
      name,
      files,
      avgLinesPerFile,
      depth,
      subtrees,
      avgOutDegree,
      asyncImportRatio,
      generateDot,
    });
    times.push(time);
    if (crashed) crashedCount++;
    if (i === 0 && copyGraphDir) {
      const src = `${name}/graph.png`;
      const dst = `${copyGraphDir}/${baseName}.png`;
      graphExists = await copyGraphIfExists(src, dst);
      if (graphExists) graph = dst;
    }
    await $`rm -rf ${name}`;
  }
  const avg = times.reduce((a, b) => a + b, 0) / times.length;
  const variance =
    times.reduce((a, b) => a + (b - avg) * (b - avg), 0) / times.length;
  const std = Math.sqrt(variance);
  return {avg, std, crashedCount, samples: times, graph, graphExists};
}

async function findParamForTarget({
  initial,
  min = 1,
  max = 1_000_000_000,
  getConfig,
  getName,
  targetMs = TARGET_10S,
  onMeasure, // optional callback to record each calibration measurement
}) {
  async function measureForParam(param) {
    const name = getName(param);
    const existing = getEntryByName(resultsFiles, name);
    if (existing) {
      if (onMeasure) onMeasure(param, existing.time, existing.crashed);
      return {avg: existing.time, crashed: !!existing.crashed};
    }
    const cfg = getConfig(param);
    const res = await measurePoint({...cfg, baseName: name}, null);
    const crashed = res.crashedCount > 0;
    const avg = res.avg;
    if (onMeasure) onMeasure(param, avg, crashed);
    return {avg, crashed};
  }
  let low = null;
  let high = null;
  let param = Math.max(initial, min);
  const tol = targetMs * TOL_FRAC;
  for (let k = 0; k < 32; k++) {
    const {avg, crashed} = await measureForParam(param);
    if (!crashed && avg < targetMs - tol) {
      low = {param, avg};
      param = Math.min(max, Math.max(param + 1, Math.floor(param * 2)));
      if (param === max) break;
    } else {
      high = {param, avg, crashed};
      break;
    }
  }
  if (!low && high) {
    // search downward to find low
    param = Math.max(min, Math.floor(high.param / 2));
    for (let k = 0; k < 32 && (!low || low.param > min); k++) {
      const {avg, crashed} = await measureForParam(param);
      if (!crashed && avg < targetMs - tol) {
        low = {param, avg};
        break;
      }
      if (param <= min) break;
      param = Math.max(min, Math.floor(param / 2));
    }
  }
  if (!low) low = {param: min, avg: targetMs};
  if (!high)
    high = {
      param: Math.min(max, Math.max(low.param + 1, Math.floor(low.param * 2))),
      avg: targetMs,
    };

  // binary search
  for (let it = 0; it < 16; it++) {
    const mid = Math.min(
      max,
      Math.max(min, Math.floor((low.param + high.param) / 2)),
    );
    if (mid === low.param || mid === high.param) break;
    const {avg, crashed} = await measureForParam(mid);
    if (!crashed && Math.abs(avg - targetMs) <= tol) {
      return mid;
    }
    if (crashed || avg > targetMs) {
      high = {param: mid, avg, crashed};
    } else {
      low = {param: mid, avg};
    }
  }
  return low.param;
}

async function copyGraphIfExists(src, dst) {
  try {
    await fs.access(src);
    await fs.copyFile(src, dst);
    return true;
  } catch {
    return false;
  }
}

const ranges = {
  files: [100, MAX_FILES],
  depth: [1, 10000],
  subtrees: [1, 64],
  avgLinesPerFile: [10, 200],
};
// (steps constant removed)

const previous = await loadPreviousResults();
const resultsFiles = [...previous.files];
const resultsDepth = [...previous.depth];
const resultsSubtrees = [...previous.subtrees];
const resultsLines = [...previous.lines];
const resultsAsyncRatio = [...previous.asyncRatio];
await fs.mkdir(artifactsDir, {recursive: true});
await fs.mkdir(`${artifactsDir}/files`, {recursive: true});
await fs.mkdir(`${artifactsDir}/depth`, {recursive: true});
await fs.mkdir(`${artifactsDir}/lines`, {recursive: true});
await fs.mkdir(`${artifactsDir}/subtrees`, {recursive: true});
await fs.mkdir(`${artifactsDir}/asyncRatio`, {recursive: true});

// Warm-up runs: 10 samples at low values (10, 20, ..., 100)
const warmupIncrements = Array.from({length: 10}, (_, i) => (i + 1) * 10);

for (const inc of warmupIncrements) {
  const numFiles = inc;
  const name = `warmup-files-${numFiles}`;
  if (hasEntryByName(resultsFiles, name)) continue;
  const {time, crashed} = await runBenchmark({
    name,
    files: numFiles,
    avgLinesPerFile: 10,
    depth: Math.floor(numFiles / 2),
    avgOutDegree: 1.1,
  });
  const graphSrc = `${name}/graph.png`;
  const graphDst = `${artifactsDir}/files/${name}.png`;
  const graphExists = await copyGraphIfExists(graphSrc, graphDst);
  await $`rm -rf ${name}`;

  resultsFiles.push({
    name,
    numFiles,
    time,
    crashed,
    graph: graphExists ? graphDst : null,
    graphExists,
  });
  await writeResult();
}

for (const inc of warmupIncrements) {
  const depth = ranges.depth[0] + inc;
  const name = `warmup-depth-${depth}`;
  if (hasEntryByName(resultsDepth, name)) continue;
  const {time, crashed} = await runBenchmark({
    name,
    files: 100,
    avgLinesPerFile: 10,
    depth,
    avgOutDegree: 1.1,
  });
  const graphSrc = `${name}/graph.png`;
  const graphDst = `${artifactsDir}/depth/${name}.png`;
  const graphExists = await copyGraphIfExists(graphSrc, graphDst);
  await $`rm -rf ${name}`;

  resultsDepth.push({
    name,
    depth,
    time,
    crashed,
    graph: graphExists ? graphDst : null,
    graphExists,
  });
  await writeResult();
}

for (const inc of warmupIncrements) {
  const avgLinesPerFile = ranges.avgLinesPerFile[0] + inc;
  const name = `warmup-avg-lines-per-file-${avgLinesPerFile}`;
  if (hasEntryByName(resultsLines, name)) continue;
  const {time, crashed} = await runBenchmark({
    name,
    files: 10,
    avgLinesPerFile,
    depth: 1,
    avgOutDegree: 1.1,
  });
  const graphSrc = `${name}/graph.png`;
  const graphDst = `${artifactsDir}/lines/${name}.png`;
  const graphExists = await copyGraphIfExists(graphSrc, graphDst);
  await $`rm -rf ${name}`;

  resultsLines.push({
    name,
    avgLinesPerFile,
    time,
    crashed,
    graph: graphExists ? graphDst : null,
    graphExists,
  });
  await writeResult();
}

for (const inc of warmupIncrements) {
  const subtrees = ranges.subtrees[0] + Math.floor(inc / 10);
  const name = `warmup-subtrees-${subtrees}`;
  if (hasEntryByName(resultsSubtrees, name)) continue;
  const {time, crashed} = await runBenchmark({
    name,
    files: 100,
    avgLinesPerFile: 10,
    depth: 10,
    subtrees,
    avgOutDegree: 1.1,
  });
  const graphSrc = `${name}/graph.png`;
  const graphDst = `${artifactsDir}/subtrees/${name}.png`;
  const graphExists = await copyGraphIfExists(graphSrc, graphDst);
  await $`rm -rf ${name}`;

  resultsSubtrees.push({
    name,
    subtrees,
    time,
    crashed,
    graph: graphExists ? graphDst : null,
    graphExists,
  });
  await writeResult();
}

for (const inc of warmupIncrements) {
  const asyncImportRatio = Math.min(1, inc / 200); // 0.05, 0.1, ..., up to 0.5
  const name = `warmup-async-ratio-${asyncImportRatio}`;
  if (hasEntryByName(resultsAsyncRatio, name)) continue;
  const {time, crashed} = await runBenchmark({
    name,
    files: 100,
    avgLinesPerFile: 10,
    depth: 10,
    subtrees: 5,
    avgOutDegree: 1.1,
    asyncImportRatio,
  });
  const graphSrc = `${name}/graph.png`;
  const graphDst = `${artifactsDir}/asyncRatio/${name}.png`;
  const graphExists = await copyGraphIfExists(graphSrc, graphDst);
  await $`rm -rf ${name}`;

  resultsAsyncRatio.push({
    name,
    asyncImportRatio,
    time,
    crashed,
    graph: graphExists ? graphDst : null,
    graphExists,
  });
  await writeResult();
}

// Calibrate file count and sweep
let fileCount10s = (() => {
  // resume: try to infer from previous calibration points
  const calibs = resultsFiles.filter(
    (r) => typeof r.numFiles === 'number' && String(r.name || '').startsWith('calibrate-files-'),
  );
  if (calibs.length) {
    calibs.sort((a, b) => Math.abs(a.time - TARGET_10S) - Math.abs(b.time - TARGET_10S));
    return calibs[0].numFiles;
  }
  return null;
})();
if (fileCount10s == null) {
  fileCount10s = await findParamForTarget({
  initial: 1000,
  min: 100,
  max: MAX_FILES,
  targetMs: TARGET_10S,
  getConfig: (files) => ({
    files,
    avgLinesPerFile: 10,
    depth: Math.max(1, Math.floor(files / 2)),
    subtrees: Math.max(1, Math.floor(files / 10)),
    avgOutDegree: 1.1,
  }),
    getName: (files) => `calibrate-files-${files}`,
    onMeasure: (param, avg, crashed) => {
      resultsFiles.push({name: `calibrate-files-${param}`, numFiles: param, time: avg, crashed, graph: null, graphExists: false});
    },
  });
}
// Clamp calibrated/resumed value to MAX_FILES
fileCount10s = Math.min(MAX_FILES, fileCount10s);
for (let scale of [0.5, 0.75, 1, 1.5, 2, 3, 4, 6, 8, 10]) {
  const numFiles = Math.min(MAX_FILES, Math.max(1, Math.floor(fileCount10s * scale)));
  const name = `benchmark-file-count-${numFiles}`;
  if (hasEntryByName(resultsFiles, name)) continue;
  const res = await measurePoint(
    {
      baseName: name,
      files: numFiles,
      avgLinesPerFile: 10,
      depth: Math.max(1, Math.floor(numFiles / 2)),
      subtrees: Math.max(1, Math.floor(numFiles / 10)),
      avgOutDegree: 1.1,
    },
    null,
  );
  resultsFiles.push({
    name,
    numFiles,
    time: res.avg,
    std: res.std,
    crashed: res.crashedCount > 0,
    samples: res.samples,
    graph: res.graph,
    graphExists: res.graphExists,
  });
  await writeResult();
}

const baselineDepth = Math.max(1, Math.floor(fileCount10s / 2));
for (let scale of [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4, 6, 8]) {
  const depth = Math.max(1, Math.floor(baselineDepth * scale));
  if (depth > fileCount10s) continue;
  const name = `benchmark-depth-${depth}`;
  if (hasEntryByName(resultsDepth, name)) continue;
  const res = await measurePoint(
    {
      baseName: name,
      files: fileCount10s,
      avgLinesPerFile: 10,
      depth,
      subtrees: 1,
      avgOutDegree: 1.1,
    },
    null,
  );
  resultsDepth.push({
    name,
    depth,
    time: res.avg,
    std: res.std,
    crashed: res.crashedCount > 0,
    samples: res.samples,
    graph: res.graph,
    graphExists: res.graphExists,
  });
  await writeResult();
}

const baselineSubtrees = Math.max(1, Math.floor(fileCount10s / 10));
for (let scale of [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4, 6, 8]) {
  const subtrees = Math.max(1, Math.floor(baselineSubtrees * scale));
  if (subtrees > fileCount10s) continue;
  const name = `benchmark-subtrees-${subtrees}`;
  if (hasEntryByName(resultsSubtrees, name)) continue;
  const res = await measurePoint(
    {
      baseName: name,
      files: fileCount10s * 2,
  avgLinesPerFile: 10,
      depth: Math.max(1, Math.floor(fileCount10s)),
      subtrees,
      avgOutDegree: 1.1,
    },
    null,
  );
  resultsSubtrees.push({
    name,
    subtrees,
    time: res.avg,
    std: res.std,
    crashed: res.crashedCount > 0,
    samples: res.samples,
    graph: res.graph,
    graphExists: res.graphExists,
  });
  await writeResult();
}

const baselineLines = 10;
for (let scale of [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4, 6, 8]) {
  const avgLinesPerFile = Math.max(1, Math.floor(baselineLines * scale));
  const name = `benchmark-avg-lines-per-file-${avgLinesPerFile}`;
  if (hasEntryByName(resultsLines, name)) continue;
  const res = await measurePoint(
    {
      baseName: name,
      files: fileCount10s * 2,
      avgLinesPerFile,
      depth: Math.max(1, Math.floor(fileCount10s)),
      subtrees: Math.max(1, Math.floor(fileCount10s)),
      avgOutDegree: 1.1,
    },
    null,
  );
  resultsLines.push({
    name,
    avgLinesPerFile,
    time: res.avg,
    std: res.std,
    crashed: res.crashedCount > 0,
    samples: res.samples,
    graph: res.graph,
    graphExists: res.graphExists,
  });
  await writeResult();
}

// Async import ratio sweep (1/20 to 1)
for (let ratio of [0.05, 0.1, 0.2, 0.4, 0.6, 0.8, 1.0]) {
  const name = `benchmark-async-ratio-${ratio}`;
  if (hasEntryByName(resultsAsyncRatio, name)) continue;
  const res = await measurePoint(
    {
      baseName: name,
      files: fileCount10s * 2,
      avgLinesPerFile: 10,
      depth: Math.max(1, Math.floor(fileCount10s / 2)),
      subtrees: Math.max(1, Math.floor(fileCount10s / 10)),
      avgOutDegree: 1.1,
      asyncImportRatio: ratio,
    },
    null,
  );
  resultsAsyncRatio.push({
    name,
    asyncImportRatio: ratio,
    time: res.avg,
    std: res.std,
    crashed: res.crashedCount > 0,
    samples: res.samples,
    graph: res.graph,
    graphExists: res.graphExists,
  });
  await writeResult();
}

await writeResult();

async function writeResult() {
  // Build a simple HTML page with charts using Chart.js
  function buildChartHtml({files, depth, subtrees, lines, asyncRatio}) {
    function aggregateAndSort(items, key) {
      const map = new Map();
      for (const it of items) {
        const x = it[key];
        const y = it.time;
        if (x == null || y == null || Number.isNaN(x) || Number.isNaN(y)) continue;
        const acc = map.get(x) || {sum: 0, count: 0};
        acc.sum += y;
        acc.count += 1;
        map.set(x, acc);
      }
      const xs = Array.from(map.keys()).sort((a, b) => a - b);
      const ys = xs.map((x) => map.get(x).sum / map.get(x).count);
      return [xs, ys];
    }

    const [filesLabels, filesTimes] = aggregateAndSort(files, 'numFiles');
    const [depthLabels, depthTimes] = aggregateAndSort(depth, 'depth');
    const [subtreesLabels, subtreesTimes] = aggregateAndSort(subtrees, 'subtrees');
    const [linesLabels, linesTimes] = aggregateAndSort(lines, 'avgLinesPerFile');
    const [asyncLabels, asyncTimes] = aggregateAndSort(asyncRatio, 'asyncImportRatio');
    const filesImgs = files
      .filter((d) => d.graphExists && d.graph)
      .map(
        (d) =>
          `<figure><img src="${d.graph}" alt="${d.name}" /><figcaption>${d.name}</figcaption></figure>`,
      )
      .join('');
    const depthImgs = depth
      .filter((d) => d.graphExists && d.graph)
      .map(
        (d) =>
          `<figure><img src="${d.graph}" alt="${d.name}" /><figcaption>${d.name}</figcaption></figure>`,
      )
      .join('');
    const subtreesImgs = subtrees
      .filter((d) => d.graphExists && d.graph)
      .map(
        (d) =>
          `<figure><img src="${d.graph}" alt="${d.name}" /><figcaption>${d.name}</figcaption></figure>`,
      )
      .join('');
    const linesImgs = lines
      .filter((d) => d.graphExists && d.graph)
      .map(
        (d) =>
          `<figure><img src="${d.graph}" alt="${d.name}" /><figcaption>${d.name}</figcaption></figure>`,
      )
      .join('');
    const asyncImgs = asyncRatio
      .filter((d) => d.graphExists && d.graph)
      .map(
        (d) =>
          `<figure><img src="${d.graph}" alt="${d.name}" /><figcaption>${d.name}</figcaption></figure>`,
      )
      .join('');

    return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Atlaspack Benchmark Results</title>
    <meta http-equiv="refresh" content="5" />
    <style>
      body { font-family: system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial, sans-serif; padding: 24px; color: #111; }
      h1 { margin: 0 0 24px; font-size: 20px; }
      .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 24px; }
      .card { border: 1px solid #e5e7eb; border-radius: 8px; padding: 16px; box-shadow: 0 1px 2px rgba(0,0,0,0.03); }
      .chart { position: relative; width: 100%; height: 280px; }
      .chart canvas { width: 100% !important; height: 100% !important; display: block; }
      .subtitle { color: #6b7280; font-size: 12px; margin-top: 4px; }
      .thumbs { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 12px; margin-top: 12px; }
      figure { margin: 0; }
      figure img { width: 100%; height: 160px; max-height: 160px; object-fit: contain; border: 1px solid #e5e7eb; border-radius: 6px; display: block; }
      figcaption { font-size: 11px; color: #6b7280; margin-top: 4px; }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  </head>
  <body>
    <h1>Atlaspack Benchmark Results (ms)</h1>
    <div class="grid">
      <div class="card">
        <strong>Build time vs Number of files</strong>
        <div class="subtitle">Time (ms) by file count</div>
        <div class="chart"><canvas id="chartFiles"></canvas></div>
        <div class="thumbs">${filesImgs}</div>
      </div>
      <div class="card">
        <strong>Build time vs Depth</strong>
        <div class="subtitle">Time (ms) by depth</div>
        <div class="chart"><canvas id="chartDepth"></canvas></div>
        <div class="thumbs">${depthImgs}</div>
      </div>
      <div class="card">
        <strong>Build time vs Subtrees</strong>
        <div class="subtitle">Time (ms) by subtrees</div>
        <div class="chart"><canvas id="chartSubtrees"></canvas></div>
        <div class="thumbs">${subtreesImgs}</div>
      </div>
      <div class="card">
        <strong>Build time vs Async import ratio</strong>
        <div class="subtitle">Time (ms) by async ratio</div>
        <div class="chart"><canvas id="chartAsync"></canvas></div>
        <div class="thumbs">${asyncImgs}</div>
      </div>
      <div class="card">
        <strong>Build time vs Avg lines per file</strong>
        <div class="subtitle">Time (ms) by avg lines</div>
        <div class="chart"><canvas id="chartLines"></canvas></div>
        <div class="thumbs">${linesImgs}</div>
      </div>
    </div>
    <script>
      const filesLabels = ${JSON.stringify(filesLabels)};
      const filesTimes = ${JSON.stringify(filesTimes)};
      const depthLabels = ${JSON.stringify(depthLabels)};
      const depthTimes = ${JSON.stringify(depthTimes)};
      const linesLabels = ${JSON.stringify(linesLabels)};
      const linesTimes = ${JSON.stringify(linesTimes)};
      const subtreesLabels = ${JSON.stringify(subtreesLabels)};
      const subtreesTimes = ${JSON.stringify(subtreesTimes)};
      const asyncLabels = ${JSON.stringify(asyncLabels)};
      const asyncTimes = ${JSON.stringify(asyncTimes)};

      function mkChart(ctx, xs, ys, label) {
        const points = xs.map((x, i) => ({x, y: ys[i]}));
        return new Chart(ctx, {
          type: 'line',
          data: {
            datasets: [{
              label,
              data: points,
              borderColor: '#2563eb',
              backgroundColor: 'rgba(37, 99, 235, 0.15)',
              tension: 0.25,
              pointRadius: 2,
              fill: true,
              parsing: false,
            }],
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
              x: { type: 'linear', title: { display: true, text: 'X' } },
              y: { type: 'linear', title: { display: true, text: 'Time (ms)' }, beginAtZero: true },
            },
            plugins: { legend: { display: false } },
          },
        });
      }

      mkChart(document.getElementById('chartFiles'), filesLabels, filesTimes, 'ms');
      mkChart(document.getElementById('chartDepth'), depthLabels, depthTimes, 'ms');
      mkChart(document.getElementById('chartSubtrees'), subtreesLabels, subtreesTimes, 'ms');
      mkChart(document.getElementById('chartLines'), linesLabels, linesTimes, 'ms');
      mkChart(document.getElementById('chartAsync'), asyncLabels, asyncTimes, 'ms');
    </script>
  </body>
</html>`;
  }

  await fs.writeFile(
    `${artifactsDir}/results.json`,
    JSON.stringify({files: resultsFiles, depth: resultsDepth, subtrees: resultsSubtrees, lines: resultsLines, asyncRatio: resultsAsyncRatio}, null, 2),
    'utf8',
  );

  const html = buildChartHtml({files: resultsFiles, depth: resultsDepth, subtrees: resultsSubtrees, lines: resultsLines, asyncRatio: resultsAsyncRatio});
  await fs.writeFile('benchmark-results.html', html, 'utf8');
  console.log('Wrote benchmark-results.html');
}
