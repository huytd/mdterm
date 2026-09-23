import { test, expect } from '@playwright/test';

function mulberry32(seed) {
  return function () {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

test.describe('L3 Visual & Geometry Harness', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/tests/terminal/visual/harness.html');
    await page.waitForFunction(() => window.__harnessReady === true);
  });

  // (a) Geometry tests: 20 seeded random sizes x lineHeight 1.0/1.2/1.5 x renderer dom (& webgl if available)
  const lineHeights = [1.0, 1.2, 1.5];
  const prng = mulberry32(1337);

  const testSizes = [];
  for (let s = 1; s <= 20; s++) {
    const width = Math.floor(prng() * 500) + 450; // 450 to 950
    const height = Math.floor(prng() * 400) + 300; // 300 to 700
    testSizes.push({ s, width, height });
  }

  test('geometry: 20 seeded random sizes x lineHeights x renderer', async ({ page }) => {
    // Check if webgl is available
    const initTest = await page.evaluate(async () => {
      const res = await window.initHarness({ width: 600, height: 400, renderer: 'webgl' });
      return res.renderer;
    });

    const renderers = ['dom'];
    if (initTest === 'webgl') {
      renderers.push('webgl');
    }

    for (const renderer of renderers) {
      for (const lh of lineHeights) {
        for (const { s, width, height } of testSizes) {
          await page.evaluate(async ({ w, h, r, l }) => {
            await window.initHarness({
              width: w,
              height: h,
              renderer: r,
              lineHeight: l,
            });
          }, { w: width, h: height, r: renderer, l: lh });

          const g = await page.evaluate(() => window.getGeometry());
          expect(g).not.toBeNull();

          // 1. .xterm-screen rect must lie fully inside .terminal-container rect
          expect(g.screenRect.left).toBeGreaterThanOrEqual(g.containerRect.left - 1.0);
          expect(g.screenRect.right).toBeLessThanOrEqual(g.containerRect.right + 1.0);
          expect(g.screenRect.top).toBeGreaterThanOrEqual(g.containerRect.top - 1.0);
          expect(g.screenRect.bottom).toBeLessThanOrEqual(g.containerRect.bottom + 1.0);

          // 2. Last row must lie inside .terminal-container rect
          if (g.lastRowRect) {
            expect(g.lastRowRect.bottom).toBeLessThanOrEqual(g.containerRect.bottom + 1.0);
          }

          // 3. cols/rows must equal floor of available content size / cell size
          const availableW = g.containerWidth - g.padX - g.scrollbarWidth;
          const availableH = g.containerHeight - g.padY;
          const expectedCols = Math.max(2, Math.floor(availableW / g.cellW));
          const expectedRows = Math.max(1, Math.floor(availableH / g.cellH));

          expect(g.cols).toBe(expectedCols);
          expect(g.rows).toBe(expectedRows);
        }
      }
    }
  });

  // (b) Resize burst: 30 container size changes within 200ms produce exactly one onResize callback
  test('resize burst: 30 container size changes within 200ms produce exactly one onResize', async ({ page }) => {
    await page.evaluate(async () => {
      await window.initHarness({ width: 600, height: 400, renderer: 'dom' });
    });

    await page.waitForTimeout(100);

    // Fire 30 size changes across ~150ms
    await page.evaluate(async () => {
      window._resizeCallbacks = [];
      for (let i = 1; i <= 30; i++) {
        window.setContainerSize(600 + i * 5, 400 + i * 3);
        await new Promise((r) => setTimeout(r, 5));
      }
    });

    // Wait for trailing debounce (50ms trailing debounce + rAF + margin)
    await page.waitForTimeout(300);

    const callbacks = await page.evaluate(() => window._resizeCallbacks);
    expect(callbacks.length).toBe(1);

    const finalGeom = await page.evaluate(() => window.getGeometry());
    expect(callbacks[0].cols).toBe(finalGeom.cols);
    expect(callbacks[0].rows).toBe(finalGeom.rows);
  });

  // (c) Screenshot goldens of box-drawing, powerline, wide-chars.
  // Only the DOM renderer (the default) is golden-tested: WebGL output under
  // headless WebKit is nondeterministic (blank frames, missing custom glyphs),
  // so a WebGL golden would be either flaky or meaningless.
  const fixtures = ['box-drawing', 'powerline', 'wide-chars'];
  const testRenderers = ['dom'];

  for (const renderer of testRenderers) {
    for (const fixture of fixtures) {
      test(`screenshot: ${fixture} with renderer ${renderer}`, async ({ page }) => {
        await page.evaluate(async ({ r }) => {
          await window.initHarness({
            width: 720,
            height: 480,
            renderer: r,
            lineHeight: 1.2,
          });
        }, { r: renderer });

        await page.evaluate(async (f) => {
          await window.replayFixture(f);
        }, fixture);

        await page.waitForTimeout(150);

        const container = page.locator('.terminal-container');
        await expect(container).toHaveScreenshot(`${fixture}-${renderer}.png`, {
          maxDiffPixelRatio: 0.01,
        });
      });
    }
  }
});
