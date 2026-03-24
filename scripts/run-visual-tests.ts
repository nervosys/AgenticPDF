/**
 * Visual Testing Runner
 * Automates comparison between AgenticPDF and native PDF viewers
 */

import { chromium, Browser, Page } from 'playwright';
import * as fs from 'fs';
import * as path from 'path';
import { PNG } from 'pngjs';
import pixelmatch from 'pixelmatch';

interface TestResult {
  pdfName: string;
  pageNumber: number;
  passed: boolean;
  pixelsDifferent: number;
  percentDifferent: number;
  screenshotPaths: {
    modern: string;
    native: string;
    diff: string;
  };
  textAnalysis: {
    totalItems: number;
    validPositions: number;
    uniqueYPositions: number;
    issues: string[];
  };
}

class VisualTestRunner {
  private browser: Browser | null = null;
  private results: TestResult[] = [];
  private outputDir: string;

  constructor(outputDir: string = './tests/visual-test-results') {
    this.outputDir = outputDir;
    this.setupDirectories();
  }

  private setupDirectories() {
    const dirs = [
      this.outputDir,
      path.join(this.outputDir, 'screenshots'),
      path.join(this.outputDir, 'diffs'),
      path.join(this.outputDir, 'reports')
    ];

    dirs.forEach(dir => {
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
    });
  }

  async initialize() {
    this.browser = await chromium.launch({
      headless: false,
      args: ['--start-maximized']
    });
    console.log('✅ Browser launched');
  }

  async close() {
    if (this.browser) {
      await this.browser.close();
      console.log('✅ Browser closed');
    }
  }

  async testPDF(pdfName: string, pageNumber: number = 1): Promise<TestResult> {
    if (!this.browser) {
      throw new Error('Browser not initialized');
    }

    console.log(`\n🔍 Testing ${pdfName} - Page ${pageNumber}`);

    const context = await this.browser.newContext({
      viewport: { width: 1400, height: 1800 }
    });

    try {
      // Test AgenticPDF viewer
      const modernPage = await context.newPage();
      await modernPage.goto('http://localhost:3031/layout-comparison.html');
      await modernPage.waitForLoadState('networkidle');

      console.log('  Loading PDF in AgenticPDF viewer...');
      await modernPage.click('button:has-text("Load PDF & Compare")');
      await modernPage.waitForTimeout(4000);

      // Capture screenshot
      const modernScreenshot = path.join(
        this.outputDir,
        'screenshots',
        `${pdfName}-modern-p${pageNumber}.png`
      );
      await modernPage.screenshot({ path: modernScreenshot });
      console.log(`  ✓ AgenticPDF screenshot saved`);

      // Analyze text layer
      const textAnalysis = await this.analyzeTextLayer(modernPage);
      console.log(`  ✓ Text analysis complete: ${textAnalysis.totalItems} items`);

      // Test native PDF viewer
      const nativePage = await context.newPage();
      const pdfPath = path.join(__dirname, '../demos', pdfName);
      await nativePage.goto(`file://${pdfPath}`);
      await nativePage.waitForTimeout(3000);

      const nativeScreenshot = path.join(
        this.outputDir,
        'screenshots',
        `${pdfName}-native-p${pageNumber}.png`
      );
      await nativePage.screenshot({ path: nativeScreenshot });
      console.log(`  ✓ Native PDF screenshot saved`);

      // Compare screenshots
      const diffPath = path.join(
        this.outputDir,
        'diffs',
        `${pdfName}-diff-p${pageNumber}.png`
      );

      const comparison = this.compareImages(
        modernScreenshot,
        nativeScreenshot,
        diffPath
      );

      console.log(`  ✓ Comparison complete: ${comparison.percentDifferent.toFixed(2)}% different`);

      const result: TestResult = {
        pdfName,
        pageNumber,
        passed: comparison.percentDifferent < 5 && textAnalysis.validPositions > 0.95,
        pixelsDifferent: comparison.pixelsDifferent,
        percentDifferent: comparison.percentDifferent,
        screenshotPaths: {
          modern: modernScreenshot,
          native: nativeScreenshot,
          diff: diffPath
        },
        textAnalysis
      };

      this.results.push(result);

      await modernPage.close();
      await nativePage.close();

      return result;

    } finally {
      await context.close();
    }
  }

  private compareImages(
    img1Path: string,
    img2Path: string,
    diffPath: string
  ): { pixelsDifferent: number; percentDifferent: number } {
    const img1 = PNG.sync.read(fs.readFileSync(img1Path));
    const img2 = PNG.sync.read(fs.readFileSync(img2Path));

    const { width, height } = img1;
    const diff = new PNG({ width, height });

    const pixelsDifferent = pixelmatch(
      img1.data,
      img2.data,
      diff.data,
      width,
      height,
      { threshold: 0.1 }
    );

    fs.writeFileSync(diffPath, PNG.sync.write(diff));

    const totalPixels = width * height;
    const percentDifferent = (pixelsDifferent / totalPixels) * 100;

    return { pixelsDifferent, percentDifferent };
  }

  private async analyzeTextLayer(page: Page) {
    return await page.evaluate(() => {
      const textLayer = document.querySelector('.text-layer');
      if (!textLayer) {
        return {
          totalItems: 0,
          validPositions: 0,
          uniqueYPositions: 0,
          issues: ['No text layer found']
        };
      }

      const textSpans = textLayer.querySelectorAll('span[role="presentation"]');
      const yPositions = new Set<number>();
      const issues: string[] = [];
      let validPositions = 0;

      textSpans.forEach((span: HTMLSpanElement, idx) => {
        const style = window.getComputedStyle(span);
        const left = parseFloat(style.left || '0');
        const top = parseFloat(style.top || '0');
        const fontSize = parseFloat(style.fontSize || '0');

        yPositions.add(Math.round(top));

        if (left >= 0 && top >= 0 && fontSize > 0) {
          validPositions++;
        } else {
          issues.push(`Item ${idx}: Invalid (${left.toFixed(1)}, ${top.toFixed(1)}, ${fontSize.toFixed(1)}px)`);
        }
      });

      return {
        totalItems: textSpans.length,
        validPositions: validPositions / textSpans.length,
        uniqueYPositions: yPositions.size,
        issues: issues.slice(0, 5) // Limit issues
      };
    });
  }

  generateReport() {
    const reportPath = path.join(
      this.outputDir,
      'reports',
      `report-${Date.now()}.html`
    );

    const passed = this.results.filter(r => r.passed).length;
    const total = this.results.length;
    const passRate = (passed / total) * 100;

    const html = `
<!DOCTYPE html>
<html>
<head>
  <title>AgenticPDF Visual Test Report</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #1a1a1a; color: #e0e0e0; padding: 20px; }
    .container { max-width: 1400px; margin: 0 auto; }
    h1 { color: #4CAF50; margin-bottom: 20px; }
    .summary { background: #2a2a2a; padding: 20px; border-radius: 8px; margin-bottom: 30px; }
    .summary h2 { color: #4CAF50; margin-bottom: 15px; }
    .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin-top: 15px; }
    .stat { background: #333; padding: 15px; border-radius: 4px; }
    .stat-label { color: #888; font-size: 12px; text-transform: uppercase; }
    .stat-value { color: #4CAF50; font-size: 28px; font-weight: bold; margin-top: 5px; }
    .test-result { background: #2a2a2a; padding: 20px; border-radius: 8px; margin-bottom: 20px; border-left: 4px solid #4CAF50; }
    .test-result.failed { border-left-color: #f44336; }
    .test-result h3 { margin-bottom: 15px; }
    .test-result.failed h3 { color: #f44336; }
    .screenshots { display: grid; grid-template-columns: repeat(3, 1fr); gap: 15px; margin-top: 15px; }
    .screenshot { text-align: center; }
    .screenshot img { max-width: 100%; border-radius: 4px; border: 1px solid #444; }
    .screenshot-label { margin-top: 8px; font-size: 12px; color: #888; }
    .metrics { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; margin: 15px 0; }
    .metric { background: #333; padding: 10px; border-radius: 4px; }
    .issues { background: #3a2a2a; padding: 15px; border-radius: 4px; margin-top: 15px; }
    .issues ul { list-style: none; }
    .issues li { padding: 5px 0; color: #ff9800; }
  </style>
</head>
<body>
  <div class="container">
    <h1>🔍 AgenticPDF Visual Test Report</h1>
    <p>Generated: ${new Date().toLocaleString()}</p>
    
    <div class="summary">
      <h2>📊 Test Summary</h2>
      <div class="stats">
        <div class="stat">
          <div class="stat-label">Total Tests</div>
          <div class="stat-value">${total}</div>
        </div>
        <div class="stat">
          <div class="stat-label">Passed</div>
          <div class="stat-value" style="color: #4CAF50;">${passed}</div>
        </div>
        <div class="stat">
          <div class="stat-label">Failed</div>
          <div class="stat-value" style="color: #f44336;">${total - passed}</div>
        </div>
        <div class="stat">
          <div class="stat-label">Pass Rate</div>
          <div class="stat-value">${passRate.toFixed(1)}%</div>
        </div>
      </div>
    </div>

    ${this.results.map(result => `
      <div class="test-result ${result.passed ? 'passed' : 'failed'}">
        <h3>${result.passed ? '✅' : '❌'} ${result.pdfName} - Page ${result.pageNumber}</h3>
        
        <div class="metrics">
          <div class="metric">
            <strong>Visual Difference:</strong> ${result.percentDifferent.toFixed(2)}%<br>
            <small>${result.pixelsDifferent.toLocaleString()} pixels different</small>
          </div>
          <div class="metric">
            <strong>Text Items:</strong> ${result.textAnalysis.totalItems}<br>
            <small>Valid: ${(result.textAnalysis.validPositions * 100).toFixed(1)}%</small>
          </div>
        </div>

        <div class="screenshots">
          <div class="screenshot">
            <img src="${path.relative(path.dirname(reportPath), result.screenshotPaths.modern)}" alt="AgenticPDF">
            <div class="screenshot-label">AgenticPDF Viewer</div>
          </div>
          <div class="screenshot">
            <img src="${path.relative(path.dirname(reportPath), result.screenshotPaths.native)}" alt="Native">
            <div class="screenshot-label">Native PDF Viewer</div>
          </div>
          <div class="screenshot">
            <img src="${path.relative(path.dirname(reportPath), result.screenshotPaths.diff)}" alt="Diff">
            <div class="screenshot-label">Pixel Difference</div>
          </div>
        </div>

        ${result.textAnalysis.issues.length > 0 ? `
          <div class="issues">
            <strong>⚠️ Issues Found:</strong>
            <ul>
              ${result.textAnalysis.issues.map(issue => `<li>${issue}</li>`).join('')}
            </ul>
          </div>
        ` : ''}
      </div>
    `).join('')}
  </div>
</body>
</html>
    `;

    fs.writeFileSync(reportPath, html);
    console.log(`\n📄 Report generated: ${reportPath}`);

    return reportPath;
  }

  printSummary() {
    console.log('\n' + '='.repeat(70));
    console.log('📊 VISUAL TEST SUMMARY');
    console.log('='.repeat(70));

    const passed = this.results.filter(r => r.passed).length;
    const total = this.results.length;

    console.log(`Total tests: ${total}`);
    console.log(`Passed: ${passed} ✅`);
    console.log(`Failed: ${total - passed} ❌`);
    console.log(`Pass rate: ${((passed / total) * 100).toFixed(1)}%`);

    console.log('\nDetailed Results:');
    this.results.forEach(result => {
      const status = result.passed ? '✅' : '❌';
      console.log(`  ${status} ${result.pdfName} (Page ${result.pageNumber})`);
      console.log(`     Visual diff: ${result.percentDifferent.toFixed(2)}%`);
      console.log(`     Text items: ${result.textAnalysis.totalItems}, Valid: ${(result.textAnalysis.validPositions * 100).toFixed(1)}%`);
    });

    console.log('='.repeat(70) + '\n');
  }
}

// Main execution
async function main() {
  const runner = new VisualTestRunner();

  try {
    await runner.initialize();

    // Test sample PDF
    await runner.testPDF('sample.pdf', 1);

    // Add more PDFs here as needed
    // await runner.testPDF('another.pdf', 1);

    runner.printSummary();
    const reportPath = runner.generateReport();

    console.log(`\n🎉 Testing complete!`);
    console.log(`📄 View full report: ${reportPath}`);

  } catch (error) {
    console.error('❌ Test execution failed:', error);
    process.exit(1);
  } finally {
    await runner.close();
  }
}

// Run if executed directly
if (require.main === module) {
  main().catch(console.error);
}

export { VisualTestRunner, TestResult };
