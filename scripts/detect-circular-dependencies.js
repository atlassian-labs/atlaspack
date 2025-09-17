#!/usr/bin/env node

/* eslint-disable no-console */
const path = require('path');
const {findProjects} = require('./common');

/**
 * Detects circular dependencies between @atlaspack packages
 */

class CircularDependencyDetector {
  constructor() {
    this.packages = new Map(); // name -> package info
    this.dependencies = new Map(); // name -> Set of dependencies
    this.visited = new Set();
    this.recursionStack = new Set();
    this.cycles = [];
  }

  /**
   * Load all packages and build dependency graph
   */
  loadPackages() {
    const packagesDir = path.join(__dirname, '..', 'packages');
    const projects = findProjects(packagesDir);

    console.log(`Found ${Object.keys(projects).length} packages`);

    // Build package registry
    for (const [projectPath, packageJson] of Object.entries(projects)) {
      const packageName = packageJson.name;
      if (packageName && packageName.startsWith('@atlaspack/')) {
        this.packages.set(packageName, {
          path: projectPath,
          packageJson,
          dependencies: new Set(),
        });
      }
    }

    // Build dependency graph - only @atlaspack packages
    for (const [packageName, packageInfo] of this.packages) {
      const deps = new Set();

      // Check dependencies, devDependencies, and peerDependencies
      const depTypes = ['dependencies', 'devDependencies', 'peerDependencies'];

      for (const depType of depTypes) {
        const dependencies = packageInfo.packageJson[depType] || {};
        for (const depName of Object.keys(dependencies)) {
          if (depName.startsWith('@atlaspack/') && this.packages.has(depName)) {
            deps.add(depName);
          }
        }
      }

      this.dependencies.set(packageName, deps);
      packageInfo.dependencies = deps;
    }

    console.log(
      `Built dependency graph for ${this.packages.size} @atlaspack packages`,
    );
  }

  /**
   * Detect cycles using DFS
   */
  detectCycles() {
    this.visited.clear();
    this.recursionStack.clear();
    this.cycles = [];

    for (const packageName of this.packages.keys()) {
      if (!this.visited.has(packageName)) {
        this.dfs(packageName, []);
      }
    }
  }

  /**
   * Depth-first search to detect cycles
   */
  dfs(packageName, path) {
    this.visited.add(packageName);
    this.recursionStack.add(packageName);
    path.push(packageName);

    const deps = this.dependencies.get(packageName) || new Set();

    for (const dep of deps) {
      if (!this.visited.has(dep)) {
        this.dfs(dep, [...path]);
      } else if (this.recursionStack.has(dep)) {
        // Found a cycle
        const cycleStart = path.indexOf(dep);
        const cycle = path.slice(cycleStart);
        cycle.push(dep); // Complete the cycle
        this.cycles.push(cycle);
      }
    }

    this.recursionStack.delete(packageName);
  }

  /**
   * Report detected circular dependencies
   */
  report() {
    if (this.cycles.length === 0) {
      console.log('✅ No circular dependencies detected!');
      return;
    }

    console.log(`❌ Found ${this.cycles.length} circular dependencies:\n`);

    this.cycles.forEach((cycle, index) => {
      console.log(`${index + 1}. Circular dependency:`);
      console.log(`   ${cycle.join(' → ')}`);

      // Show the actual dependency relationships
      for (let i = 0; i < cycle.length - 1; i++) {
        const from = cycle[i];
        const to = cycle[i + 1];
        const packageInfo = this.packages.get(from);

        // Find which dependency type contains this relationship
        const depTypes = [
          'dependencies',
          'devDependencies',
          'peerDependencies',
        ];
        let depType = '';

        for (const type of depTypes) {
          if (
            packageInfo.packageJson[type] &&
            packageInfo.packageJson[type][to]
          ) {
            depType = type;
            break;
          }
        }

        console.log(`     ${from} has ${to} in ${depType}`);
      }
      console.log();
    });

    console.log('💡 To fix these cycles, consider:');
    console.log('   - Moving shared code to a separate package');
    console.log(
      "   - Converting some dependencies to devDependencies if they're only used in tests",
    );
    console.log('   - Refactoring to remove the circular relationship');
    console.log(
      '   - Using dependency injection or interfaces to break the cycle',
    );
  }

  /**
   * Run the full analysis
   */
  run() {
    console.log(
      '🔍 Detecting circular dependencies between @atlaspack packages...\n',
    );

    try {
      this.loadPackages();
      this.detectCycles();
      this.report();
    } catch (error) {
      console.error('Error during analysis:', error);
      process.exitCode = 1;
    }
  }
}

// Run the detector
if (require.main === module) {
  const detector = new CircularDependencyDetector();
  detector.run();
}

module.exports = CircularDependencyDetector;
