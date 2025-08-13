#!/usr/bin/env node

import express from 'express';
import * as path from 'node:path';
import * as url from 'node:url';
import chalk from 'chalk';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const benchmarkDir = path.resolve(__dirname, '..');

const app = express();
const PORT = process.env.PORT || 3000;

// Serve static files from both dist directories
app.use('/dist-off', express.static(path.join(benchmarkDir, 'dist-off')));
app.use('/dist-on', express.static(path.join(benchmarkDir, 'dist-on')));

// Serve benchmark comparison page
app.get('/', (req, res) => {
  res.send(`
    <!DOCTYPE html>
    <html>
    <head>
        <title>Reused Inline Requires Benchmark</title>
        <style>
            body { 
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif; 
                max-width: 1400px; 
                margin: 0 auto; 
                padding: 20px; 
                line-height: 1.6;
                background: #f8f9fa;
                color: #212529;
            }
            h1 { 
                text-align: center; 
                color: #007cba; 
                margin-bottom: 10px;
                font-size: 2.5rem;
            }
            p { 
                text-align: center; 
                margin-bottom: 30px; 
                font-size: 1.1rem;
                color: #6c757d;
            }
            .comparison { 
                display: grid; 
                grid-template-columns: 1fr 1fr; 
                gap: 30px; 
                margin-top: 20px;
            }
            .version { 
                background: white;
                border: 1px solid #e9ecef; 
                padding: 20px; 
                border-radius: 12px; 
                box-shadow: 0 4px 6px rgba(0, 0, 0, 0.05);
                transition: transform 0.2s ease, box-shadow 0.2s ease;
            }
            .version:hover {
                transform: translateY(-2px);
                box-shadow: 0 8px 15px rgba(0, 0, 0, 0.1);
            }
            .version h3 { 
                margin-top: 0; 
                margin-bottom: 15px;
                font-size: 1.5rem;
                text-align: center;
                padding: 10px 0;
                border-radius: 8px;
            }
            .version h3:first-of-type { 
                background: #dc3545; 
                color: white; 
            }
            .version:last-child h3 { 
                background: #28a745; 
                color: white; 
            }
            iframe { 
                width: 100%; 
                height: 700px; 
                border: 2px solid #e9ecef; 
                border-radius: 8px;
                background: white;
            }
            .metrics { 
                background: linear-gradient(135deg, #f8f9fa, #e9ecef); 
                padding: 15px; 
                margin: 15px 0; 
                border-radius: 8px; 
                border: 1px solid #dee2e6;
                font-size: 0.9rem;
                max-height: 200px;
                overflow-y: auto;
            }
            .metrics h4 {
                margin-top: 0;
                margin-bottom: 10px;
                color: #495057;
                font-size: 1.1rem;
            }
            .metrics div {
                margin: 5px 0;
                padding: 3px 0;
                border-bottom: 1px dotted #dee2e6;
            }
            .metrics div:last-child {
                border-bottom: none;
            }
            .metrics strong {
                color: #007cba;
            }
            @media (max-width: 1200px) {
                .comparison { 
                    grid-template-columns: 1fr; 
                    gap: 20px; 
                }
                .version {
                    margin-bottom: 20px;
                }
            }
            .status-indicator {
                display: inline-block;
                width: 12px;
                height: 12px;
                border-radius: 50%;
                margin-right: 8px;
                animation: pulse 2s infinite;
            }
            .status-loading { background: #ffc107; }
            .status-ready { background: #28a745; }
            @keyframes pulse {
                0% { opacity: 1; }
                50% { opacity: 0.5; }
                100% { opacity: 1; }
            }
        </style>
    </head>
    <body>
        <h1>Reused Inline Requires Benchmark Comparison</h1>
        <p>Compare the performance of builds with the <code>reusedInlineRequires</code> feature enabled and disabled.</p>
        
        <div class="comparison">
            <div class="version">
                <h3><span class="status-indicator status-loading" id="status-off"></span>Feature OFF</h3>
                <div class="metrics" id="metrics-off">
                    <h4>Performance Metrics</h4>
                    <div>⏳ Loading metrics...</div>
                </div>
                <iframe src="/dist-off/index.html" id="frame-off" title="Benchmark with reusedInlineRequires disabled"></iframe>
            </div>
            
            <div class="version">
                <h3><span class="status-indicator status-loading" id="status-on"></span>Feature ON</h3>
                <div class="metrics" id="metrics-on">
                    <h4>Performance Metrics</h4>
                    <div>⏳ Loading metrics...</div>
                </div>
                <iframe src="/dist-on/index.html" id="frame-on" title="Benchmark with reusedInlineRequires enabled"></iframe>
            </div>
        </div>

        <script>
            let loadedFrames = { off: false, on: false };
            
            // Listen for performance metrics from iframes
            window.addEventListener('message', (event) => {
                if (event.data.type === 'PERFORMANCE_METRICS') {
                    const source = event.source;
                    const isOff = source === document.getElementById('frame-off').contentWindow;
                    const suffix = isOff ? 'off' : 'on';
                    const metricsId = 'metrics-' + suffix;
                    const statusId = 'status-' + suffix;
                    
                    // Update status indicator
                    const statusEl = document.getElementById(statusId);
                    statusEl.className = 'status-indicator status-ready';
                    loadedFrames[suffix] = true;
                    
                    const metrics = event.data.data;
                    const metricsEl = document.getElementById(metricsId);
                    
                    let html = '<h4>✅ Performance Metrics</h4>';
                    
                    // Format metrics nicely
                    for (const [key, value] of Object.entries(metrics)) {
                        if (typeof value === 'object' && value.average !== undefined) {
                            html += \`<div><strong>\${formatKey(key)}:</strong> \${value.average.toFixed(2)}ms (avg)</div>\`;
                        } else if (typeof value === 'number') {
                            html += \`<div><strong>\${formatKey(key)}:</strong> \${value.toFixed(2)}ms</div>\`;
                        } else {
                            html += \`<div><strong>\${formatKey(key)}:</strong> \${JSON.stringify(value)}</div>\`;
                        }
                    }
                    
                    metricsEl.innerHTML = html;
                }
            });
            
            // Format metric keys for display
            function formatKey(key) {
                return key.replace(/([A-Z])/g, ' $1').replace(/^./, str => str.toUpperCase());
            }
            
            // Check iframe loading status
            document.getElementById('frame-off').onload = () => {
                if (!loadedFrames.off) {
                    setTimeout(() => {
                        if (!loadedFrames.off) {
                            document.getElementById('status-off').className = 'status-indicator status-loading';
                            document.getElementById('metrics-off').innerHTML = '<h4>⚠️ Loading Issues</h4><div>Application may not be loading properly</div>';
                        }
                    }, 5000);
                }
            };
            
            document.getElementById('frame-on').onload = () => {
                if (!loadedFrames.on) {
                    setTimeout(() => {
                        if (!loadedFrames.on) {
                            document.getElementById('status-on').className = 'status-indicator status-loading';
                            document.getElementById('metrics-on').innerHTML = '<h4>⚠️ Loading Issues</h4><div>Application may not be loading properly</div>';
                        }
                    }, 5000);
                }
            };

            // Reload iframes every 60 seconds for fresh metrics
            setInterval(() => {
                loadedFrames = { off: false, on: false };
                document.getElementById('status-off').className = 'status-indicator status-loading';
                document.getElementById('status-on').className = 'status-indicator status-loading';
                document.getElementById('frame-off').src = document.getElementById('frame-off').src;
                document.getElementById('frame-on').src = document.getElementById('frame-on').src;
            }, 60000);
        </script>
    </body>
    </html>
  `);
});

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

app.listen(PORT, () => {
  console.log(chalk.green(`🚀 Benchmark server running at http://localhost:${PORT}`));
  console.log(chalk.blue(`📊 Feature OFF: http://localhost:${PORT}/dist-off/index.html`));
  console.log(chalk.blue(`📊 Feature ON:  http://localhost:${PORT}/dist-on/index.html`));
  console.log(chalk.yellow(`🔍 Comparison:  http://localhost:${PORT}/`));
});

export default app;
