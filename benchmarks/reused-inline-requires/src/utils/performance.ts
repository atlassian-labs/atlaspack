// Performance tracking utilities
class PerformanceMetrics {
  private metrics: Map<string, number[]> = new Map();

  recordMetric(name: string, value: number): void {
    if (!this.metrics.has(name)) {
      this.metrics.set(name, []);
    }
    this.metrics.get(name)!.push(value);
  }

  getMetric(name: string): number[] {
    return this.metrics.get(name) || [];
  }

  getAverageMetric(name: string): number {
    const values = this.getMetric(name);
    return values.length > 0 ? values.reduce((a, b) => a + b, 0) / values.length : 0;
  }

  getAllMetrics(): Record<string, { values: number[], average: number }> {
    const result: Record<string, { values: number[], average: number }> = {};
    
    for (const [name, values] of this.metrics.entries()) {
      result[name] = {
        values: [...values],
        average: this.getAverageMetric(name)
      };
    }
    
    return result;
  }

  clear(): void {
    this.metrics.clear();
  }
}

export const performanceMetrics = new PerformanceMetrics();

// Simple timing utility for manual measurement
export function timeFunction<T>(name: string, fn: () => T): T {
  const start = performance.now();
  const result = fn();
  const end = performance.now();
  
  performanceMetrics.recordMetric(name, end - start);
  console.log(`${name} execution time: ${end - start}ms`);
  
  return result;
}
