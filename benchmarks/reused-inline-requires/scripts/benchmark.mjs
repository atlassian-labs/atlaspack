#!/usr/bin/env node

import * as child_process from 'node:child_process';
import * as fs from 'node:fs/promises';
import * as fsSync from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import * as os from 'node:os';

import { chromium } from '@playwright/test';
import chalk from 'chalk';
import { printTable } from '@oclif/table';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const benchmarkDir = path.resolve(__dirname, '..');

const RUNS = parseInt(process.env.BENCHMARK_RUNS || '5', 10);
const BROWSER_TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || '30000', 10);
const DEBUG_MODE = process.env.DEBUG === 'true' || process.argv.includes('--debug');

let headers = 0;

class RuntimeBenchmarkRunner {
  constructor() {
    this.results = {
      bundleMetrics: { off: null, on: null },
      browserMetrics: { off: [], on: [] }
    };
  }

  async run() {
    writeHeader('Reused Inline Requires Runtime Benchmark');
    
    this.printSettings();
    
    await this.buildBothVersions();
    await this.runBrowserBenchmarks();
    
    this.generateReport();
  }

  printSettings() {
    writeHeader('Settings');
    
    printTable({
      columns: [
        { key: 'setting', name: 'Setting' },
        { key: 'value', name: 'Value' }
      ],
      data: [
        { setting: 'Runtime test runs', value: chalk.yellow(RUNS) },
        { setting: 'Browser timeout', value: chalk.yellow(BROWSER_TIMEOUT + 'ms') },
        { setting: 'Debug mode', value: DEBUG_MODE ? chalk.green('ON') : chalk.gray('OFF') },
        { setting: 'Node version', value: chalk.green(process.version) },
        { setting: 'Platform', value: chalk.green(os.platform() + ' ' + os.arch()) }
      ]
    });
  }

  async buildBothVersions() {
    writeHeader('Building Test Versions');
    
    console.log('Building version with reusedInlineRequires OFF...');
    await this.runBuild('off');
    
    console.log('Building version with reusedInlineRequires ON...');
    await this.runBuild('on');

    // Analyze bundle sizes for context
    this.results.bundleMetrics.off = await this.analyzeBundleSize('dist-off');
    this.results.bundleMetrics.on = await this.analyzeBundleSize('dist-on');
    
    console.log(chalk.green('✓ Both versions built successfully'));
  }

  async runBuild(mode) {
    const distDir = mode === 'on' ? 'dist-on' : 'dist-off';
    
    try {
      // Clean previous build
      await this.cleanDirectory(distDir);
      
      // Run build
      child_process.execFileSync(
        'npx',
        [
          'atlaspack',
          'build',
          '--no-cache',
          '--feature-flag',
          `reusedInlineRequires=${mode === 'on'}`,
          'src/index.html',
          '--dist-dir',
          distDir,
          '--public-url',
          `/${distDir}`
        ],
        {
          cwd: benchmarkDir,
          stdio: 'pipe'
        }
      );
    } catch (error) {
      console.error(`Build failed for mode ${mode}:`, error.message);
      throw error;
    }
  }

  async analyzeBundleSize(distDir) {
    const distPath = path.join(benchmarkDir, distDir);
    
    if (!fsSync.existsSync(distPath)) {
      return null;
    }

    const files = await fs.readdir(distPath);
    const jsFiles = files.filter(f => f.endsWith('.js'));
    
    let totalSize = 0;
    const fileDetails = [];

    for (const file of jsFiles) {
      const filePath = path.join(distPath, file);
      const stats = await fs.stat(filePath);
      fileDetails.push({
        name: file,
        size: stats.size,
        sizeKB: Math.round(stats.size / 1024 * 100) / 100
      });
      totalSize += stats.size;
    }

    return {
      totalFiles: jsFiles.length,
      totalSize,
      totalSizeKB: Math.round(totalSize / 1024 * 100) / 100,
      files: fileDetails.sort((a, b) => b.size - a.size)
    };
  }

  async runBrowserBenchmarks() {
    writeHeader('Browser Runtime Performance');
    
    const server = await this.startServer();
    
    try {
      console.log('Testing runtime performance with reusedInlineRequires OFF...');
      for (let i = 0; i < RUNS; i++) {
        const metrics = await this.runBrowserTest('off');
        this.results.browserMetrics.off.push(metrics);
        console.log(`  Run ${i + 1}: Load ${metrics.loadTime.toFixed(1)}ms, Execute ${metrics.executeTime.toFixed(1)}ms, FCP ${metrics.firstContentfulPaint.toFixed(1)}ms`);
      }

      console.log('\nTesting runtime performance with reusedInlineRequires ON...');
      for (let i = 0; i < RUNS; i++) {
        const metrics = await this.runBrowserTest('on');
        this.results.browserMetrics.on.push(metrics);
        console.log(`  Run ${i + 1}: Load ${metrics.loadTime.toFixed(1)}ms, Execute ${metrics.executeTime.toFixed(1)}ms, FCP ${metrics.firstContentfulPaint.toFixed(1)}ms`);
      }
    } finally {
      server.close();
    }
  }

  async runBrowserTest(mode) {
    const browser = await chromium.launch({ 
      headless: !DEBUG_MODE,
      devtools: DEBUG_MODE,
      slowMo: DEBUG_MODE ? 500 : 0,
      // In debug mode, remove timeout to allow manual debugging
      timeout: DEBUG_MODE ? 0 : 30000
    });
    
    try {
      const context = await browser.newContext({
        viewport: { width: 1280, height: 720 }
      });
      const page = await context.newPage();
      
      // Enable performance monitoring and memory tracking
      await page.addInitScript(() => {
        window.performanceData = {
          loadStart: Date.now(),
          featureExecutionTimes: []
        };
        
        // Override console.log to capture feature execution times
        const originalLog = console.log;
        console.log = (...args) => {
          if (args[0] && args[0].includes && args[0].includes('execution time')) {
            window.performanceData.featureExecutionTimes.push(args[0]);
          }
          originalLog.apply(console, args);
        };
      });

      const distDir = mode === 'on' ? 'dist-on' : 'dist-off';
      const url = `http://localhost:3000/${distDir}/index.html`;

      const startTime = Date.now();
      
      if (DEBUG_MODE) {
        console.log(`🔍 Debug: Navigating to ${url}`);
      }
      
      // Navigate and wait for load
      await page.goto(url, { waitUntil: 'networkidle', timeout: BROWSER_TIMEOUT });
      
      if (DEBUG_MODE) {
        console.log('🔍 Debug: Page loaded, waiting for #root element');
      }
      
      // Wait for React to render
      await page.waitForSelector('#root', { timeout: 500000 });
      
      if (DEBUG_MODE) {
        console.log('🔍 Debug: #root element found, checking if it has content');
        const rootContent = await page.locator('#root').innerHTML();
        console.log(`🔍 Debug: Root content length: ${rootContent.length} characters`);
        
        if (rootContent.length < 50) {
          console.log('⚠️  Debug: Root element seems empty, waiting longer...');
          await page.waitForTimeout(3000);
        }
        
        console.log('🔍 Debug: Opening Playwright Inspector - use page.pause() to debug interactively');
        // Pause for manual debugging with Playwright Inspector
        await page.pause();
      }
      
      // Wait a bit more for all components to initialize
      await page.waitForTimeout(DEBUG_MODE ? 2000 : 1000);
      
      // Trigger feature execution to measure runtime performance
      // await this.triggerFeatureTests(page);
      
      if (DEBUG_MODE) {
        console.log('🔍 Debug: Feature tests completed. You can now inspect the final state.');
        await page.pause();
      }
      
      // Get comprehensive performance metrics
      const metrics = await page.evaluate(() => {
        const navigation = performance.getEntriesByType('navigation')[0];
        const paint = performance.getEntriesByType('paint');
        const memory = performance.memory;
        
        // Measure feature execution performance
        const featureButton = document.querySelector('button');
        let featureExecutionTime = 0;
        
        if (featureButton) {
          const start = performance.now();
          featureButton.click();
          featureExecutionTime = performance.now() - start;
        }
        
        // Get resource loading times
        const resources = performance.getEntriesByType('resource').filter(r => r.name.includes('.js'));
        const totalResourceTime = resources.reduce((sum, r) => sum + r.duration, 0);
        
        return {
          // Core timing metrics
          loadTime: navigation.loadEventEnd - navigation.loadEventStart,
          parseTime: navigation.domContentLoadedEventEnd - navigation.domContentLoadedEventStart,
          executeTime: featureExecutionTime,
          totalLoadTime: navigation.loadEventEnd - navigation.navigationStart,
          
          // Paint metrics
          firstPaint: paint.find(p => p.name === 'first-paint')?.startTime || 0,
          firstContentfulPaint: paint.find(p => p.name === 'first-contentful-paint')?.startTime || 0,
          
          // Resource metrics
          resourceCount: resources.length,
          totalResourceTime,
          averageResourceTime: resources.length ? totalResourceTime / resources.length : 0,
          
          // Memory metrics
          jsHeapSize: memory ? memory.usedJSHeapSize : 0,
          jsHeapSizeLimit: memory ? memory.jsHeapSizeLimit : 0,
          
          // Feature execution
          featureExecutionTime,
          
          // Additional context
          totalTime: Date.now() - window.performanceData.loadStart,
          userAgent: navigator.userAgent
        };
      });

      return {
        ...metrics,
        mode,
        timestamp: new Date().toISOString()
      };
    } finally {
      await browser.close();
    }
  }

  async triggerFeatureTests(page) {
    try {
      if (DEBUG_MODE) {
        console.log('🔍 Debug: Looking for test buttons...');
        const allButtons = await page.locator('button').count();
        console.log(`🔍 Debug: Found ${allButtons} buttons total`);
      }
      
      // Click the main feature test button
      const testButton = page.locator('button:has-text("Test Feature Performance")');
      const testButtonCount = await testButton.count();
      if (DEBUG_MODE) {
        console.log(`🔍 Debug: Found ${testButtonCount} "Test Feature Performance" buttons`);
      }
      if (testButtonCount > 0) {
        await testButton.click();
        await page.waitForTimeout(100);
        if (DEBUG_MODE) console.log('🔍 Debug: Clicked main test button');
      }
      
      // Execute individual feature buttons
      const featureButtons = page.locator('button:has-text("Re-")');
      const count = await featureButtons.count();
      if (DEBUG_MODE) {
        console.log(`🔍 Debug: Found ${count} "Re-" buttons`);
      }
      for (let i = 0; i < Math.min(count, 3); i++) { // Limit to avoid timeout
        await featureButtons.nth(i).click();
        await page.waitForTimeout(50);
        if (DEBUG_MODE) console.log(`🔍 Debug: Clicked feature button ${i + 1}`);
      }
      
      if (DEBUG_MODE) {
        console.log('🔍 Debug: Finished triggering feature tests, taking screenshot...');
        await page.screenshot({ path: `debug-screenshot-${Date.now()}.png` });
      }
      
    } catch (error) {
      console.warn('Some feature tests could not be triggered:', error.message);
      if (DEBUG_MODE) {
        console.log('🔍 Debug: Error occurred, taking error screenshot...');
        await page.screenshot({ path: `debug-error-${Date.now()}.png` });
      }
    }
  }

  async startServer() {
    const { default: express } = await import('express');
    const app = express();
    
    // Serve static files from both dist directories
    app.use('/dist-off', express.static(path.join(benchmarkDir, 'dist-off')));
    app.use('/dist-on', express.static(path.join(benchmarkDir, 'dist-on')));
    
    return new Promise((resolve) => {
      const server = app.listen(3000, () => {
        console.log('Test server started on http://localhost:3000');
        resolve(server);
      });
    });
  }

  generateReport() {
    writeHeader('Runtime Performance Results');
    
    // Bundle size comparison for context
    if (this.results.bundleMetrics.off && this.results.bundleMetrics.on) {
      const bundleOffSize = this.results.bundleMetrics.off.totalSizeKB;
      const bundleOnSize = this.results.bundleMetrics.on.totalSizeKB;
      const bundleReduction = ((bundleOffSize - bundleOnSize) / bundleOffSize * 100);

      printTable({
        columns: [
          { key: 'metric', name: 'Bundle Metric' },
          { key: 'off', name: 'Feature OFF' },
          { key: 'on', name: 'Feature ON' },
          { key: 'improvement', name: 'Change' }
        ],
        data: [
          {
            metric: 'Total Bundle Size',
            off: `${bundleOffSize} KB`,
            on: `${bundleOnSize} KB`,
            improvement: `${bundleReduction > 0 ? '-' : '+'}${Math.abs(bundleReduction).toFixed(1)}%`
          },
          {
            metric: 'Number of JS Files',
            off: String(this.results.bundleMetrics.off.totalFiles),
            on: String(this.results.bundleMetrics.on.totalFiles),
            improvement: String(this.results.bundleMetrics.on.totalFiles - this.results.bundleMetrics.off.totalFiles)
          }
        ]
      });
    }

    // Browser runtime performance comparison
    const metrics = [
      { key: 'loadTime', name: 'Load Time' },
      { key: 'parseTime', name: 'Parse Time' },
      { key: 'executeTime', name: 'Execute Time' },
      { key: 'totalLoadTime', name: 'Total Load Time' },
      { key: 'firstContentfulPaint', name: 'First Contentful Paint' },
      { key: 'featureExecutionTime', name: 'Feature Execution' },
      { key: 'jsHeapSize', name: 'JS Heap Size' }
    ];

    const performanceData = metrics.map(metric => {
      const offValues = this.results.browserMetrics.off.map(m => m[metric.key]);
      const onValues = this.results.browserMetrics.on.map(m => m[metric.key]);
      
      const offAvg = this.average(offValues);
      const onAvg = this.average(onValues);
      const improvement = ((offAvg - onAvg) / offAvg * 100);
      
      let offDisplay, onDisplay;
      if (metric.key === 'jsHeapSize') {
        offDisplay = `${(offAvg / 1024 / 1024).toFixed(1)} MB`;
        onDisplay = `${(onAvg / 1024 / 1024).toFixed(1)} MB`;
      } else {
        offDisplay = `${offAvg.toFixed(1)}ms`;
        onDisplay = `${onAvg.toFixed(1)}ms`;
      }
      
      return {
        metric: metric.name,
        off: offDisplay,
        on: onDisplay,
        improvement: `${improvement > 0 ? '+' : ''}${improvement.toFixed(1)}%`
      };
    });

    printTable({
      columns: [
        { key: 'metric', name: 'Runtime Metric' },
        { key: 'off', name: 'Feature OFF' },
        { key: 'on', name: 'Feature ON' },
        { key: 'improvement', name: 'Improvement' }
      ],
      data: performanceData
    });

    // Statistical summary
    this.printStatisticalSummary();
    
    // Save detailed results
    this.saveResults();
  }

  printStatisticalSummary() {
    writeHeader('Statistical Summary');
    
    const keyMetrics = ['loadTime', 'executeTime', 'firstContentfulPaint'];
    
    keyMetrics.forEach(metric => {
      const offValues = this.results.browserMetrics.off.map(m => m[metric]);
      const onValues = this.results.browserMetrics.on.map(m => m[metric]);
      
      console.log(chalk.bold(`\n${metric}:`));
      console.log(`  Feature OFF: ${this.average(offValues).toFixed(1)}ms ± ${this.standardDeviation(offValues).toFixed(1)}ms`);
      console.log(`              P50: ${this.percentile(offValues, 50).toFixed(1)}ms, P90: ${this.percentile(offValues, 90).toFixed(1)}ms`);
      console.log(`  Feature ON:  ${this.average(onValues).toFixed(1)}ms ± ${this.standardDeviation(onValues).toFixed(1)}ms`);
      console.log(`              P50: ${this.percentile(onValues, 50).toFixed(1)}ms, P90: ${this.percentile(onValues, 90).toFixed(1)}ms`);
      
      const improvement = ((this.average(offValues) - this.average(onValues)) / this.average(offValues) * 100);
      const color = improvement > 0 ? chalk.green : chalk.red;
      console.log(`  Improvement: ${color(improvement.toFixed(1) + '%')}`);
    });
  }

  async saveResults() {
    const resultsFile = path.join(benchmarkDir, 'runtime-benchmark-results.json');
    const summary = {
      timestamp: new Date().toISOString(),
      runs: RUNS,
      bundleMetrics: this.results.bundleMetrics,
      browserMetrics: {
        off: {
          statistics: this.calculateStatistics(this.results.browserMetrics.off),
          averages: this.calculateAverages(this.results.browserMetrics.off),
          all: this.results.browserMetrics.off
        },
        on: {
          statistics: this.calculateStatistics(this.results.browserMetrics.on),
          averages: this.calculateAverages(this.results.browserMetrics.on),
          all: this.results.browserMetrics.on
        }
      },
      summary: {
        bundleSizeReduction: this.results.bundleMetrics.off && this.results.bundleMetrics.on 
          ? ((this.results.bundleMetrics.off.totalSizeKB - this.results.bundleMetrics.on.totalSizeKB) / this.results.bundleMetrics.off.totalSizeKB * 100)
          : null,
        loadTimeImprovement: this.calculateImprovement('loadTime'),
        executeTimeImprovement: this.calculateImprovement('executeTime'),
        fcpImprovement: this.calculateImprovement('firstContentfulPaint')
      }
    };

    await fs.writeFile(resultsFile, JSON.stringify(summary, null, 2));
    console.log(`\nDetailed results saved to: ${chalk.cyan(resultsFile)}`);
  }

  calculateAverages(metrics) {
    if (!metrics.length) return {};
    
    const keys = Object.keys(metrics[0]).filter(k => typeof metrics[0][k] === 'number');
    const averages = {};
    
    keys.forEach(key => {
      const values = metrics.map(m => m[key]);
      averages[key] = this.average(values);
    });
    
    return averages;
  }

  calculateStatistics(metrics) {
    if (!metrics.length) return {};
    
    const keys = Object.keys(metrics[0]).filter(k => typeof metrics[0][k] === 'number');
    const statistics = {};
    
    keys.forEach(key => {
      const values = metrics.map(m => m[key]);
      statistics[key] = {
        average: this.average(values),
        standardDeviation: this.standardDeviation(values),
        p50: this.percentile(values, 50),
        p90: this.percentile(values, 90),
        min: Math.min(...values),
        max: Math.max(...values)
      };
    });
    
    return statistics;
  }

  calculateImprovement(metric) {
    const offAvg = this.average(this.results.browserMetrics.off.map(m => m[metric]));
    const onAvg = this.average(this.results.browserMetrics.on.map(m => m[metric]));
    return ((offAvg - onAvg) / offAvg * 100);
  }

  average(array) {
    if (!array.length) return 0;
    return array.reduce((sum, val) => sum + val, 0) / array.length;
  }

  standardDeviation(array) {
    const avg = this.average(array);
    const squareDiffs = array.map(value => Math.pow(value - avg, 2));
    return Math.sqrt(this.average(squareDiffs));
  }

  percentile(array, percentile) {
    if (!array.length) return 0;
    const sorted = [...array].sort((a, b) => a - b);
    const index = (percentile / 100) * (sorted.length - 1);
    
    if (Math.floor(index) === index) {
      return sorted[index];
    } else {
      const lower = sorted[Math.floor(index)];
      const upper = sorted[Math.ceil(index)];
      const weight = index - Math.floor(index);
      return lower + (upper - lower) * weight;
    }
  }

  async cleanDirectory(dir) {
    const dirPath = path.join(benchmarkDir, dir);
    if (fsSync.existsSync(dirPath)) {
      await fs.rm(dirPath, { recursive: true, force: true });
    }
  }
}

function writeHeader(header) {
  if (headers > 0) {
    console.log('');
  }
  console.log(chalk.bold.cyan.underline(header.toUpperCase()));
  console.log('');
  headers += 1;
}

// Run the benchmark
const runner = new RuntimeBenchmarkRunner();
runner.run().catch(console.error);