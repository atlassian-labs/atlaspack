import React, { useState, useEffect, lazy, Suspense } from 'react';
import ReactDOM from 'react-dom/client';
import { performanceMetrics } from './utils/performance';
import { FeatureA, FeatureAComponent } from './tests/featureA';


const App: React.FC = () => {
  const [metrics, setMetrics] = useState<any>(null);

  useEffect(() => {
    const startTime = performance.now();
    
    // Measure performance metrics
    const bundleSize = document.querySelectorAll('script').length;
    const loadTime = performance.now() - startTime;
    
    performanceMetrics.recordMetric('pageLoad', loadTime);
    performanceMetrics.recordMetric('bundleScripts', bundleSize);
    
    setMetrics({
      loadTime: loadTime.toFixed(2),
      bundleScripts: bundleSize,
      timestamp: Date.now()
    });


    // Report performance data to parent window if in iframe
    if (window.parent !== window) {
      window.parent.postMessage({
        type: 'PERFORMANCE_METRICS',
        data: performanceMetrics.getAllMetrics()
      }, '*');
    }
  }, []);

  const handleFeatureTest = () => {
    const start = performance.now();
    
    // Exercise features that use many requires
    const featureA = new FeatureA();
    
    featureA.execute();
    
    const executionTime = performance.now() - start;
    performanceMetrics.recordMetric('featureExecution', executionTime);
    
    setMetrics(prev => ({
      ...prev,
      lastExecutionTime: executionTime.toFixed(2)
    }));
  };

  const updateMetricsDisplay = () => {
    const metricsEl = document.getElementById('load-time');
    const bundleSizeEl = document.getElementById('bundle-size');
    const executionTimeEl = document.getElementById('execution-time');
    
    if (metricsEl && metrics) {
      metricsEl.textContent = `Page Load Time: ${metrics.loadTime}ms`;
    }
    
    if (bundleSizeEl && metrics) {
      bundleSizeEl.textContent = `Bundle Scripts: ${metrics.bundleScripts}`;
    }
    
    if (executionTimeEl && metrics?.lastExecutionTime) {
      executionTimeEl.textContent = `Last Execution Time: ${metrics.lastExecutionTime}ms`;
    }
  };

  useEffect(() => {
    updateMetricsDisplay();
  }, [metrics]);

  return (
    <div>
      <h2>Interactive Feature Testing</h2>
      
      <div className="feature-section">
        <h3>Core Features</h3>
        <FeatureAComponent />
        <button onClick={handleFeatureTest}>
          Test Feature Performance
        </button>
      </div>
    </div>
  );
};

// Mount the app
const container = document.getElementById('root');
if (container) {
  const root = ReactDOM.createRoot(container);
  root.render(<App />);
}

// Export for potential programmatic access
export default App;
