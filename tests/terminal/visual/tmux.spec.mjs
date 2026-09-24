// End-to-end tests for tmux control mode: the real frontend client
// (frontend/js/tmux-client.js) driven against a real tmux server on an
// isolated socket. tmux itself is the oracle for what each pane should show.
import { test, expect } from '@playwright/test';
import { ControlConnection, killServer, tmux, tmuxAvailable, useSocket } from './tmux-backend.mjs';

const SESSION = 'e2e';

test.skip(!tmuxAvailable(), 'tmux not installed');
test.describe.configure({ mode: 'serial' });

function oracle(pane) {
  return tmux('capture-pane', '-p', '-t', `%${pane}`)
    .replace(/\n$/, '')
    .split('\n')
    .map(l => l.replace(/\s+$/, ''));
}

async function screen(page, pane) {
  const lines = await page.evaluate((p) => window.harness.screen(p), pane);
  return lines.map(l => l.replace(/\s+$/, ''));
}

function oracleCursor(pane) {
  const [x, y] = tmux('display', '-p', '-t', `%${pane}`, '#{cursor_x} #{cursor_y}').trim().split(' ').map(Number);
  return { x, y };
}

async function expectPaneMatchesTmux(page, pane) {
  await expect.poll(async () => {
    const got = { lines: await screen(page, pane), cursor: await page.evaluate((p) => window.harness.panes().find(v => v.id === p).cursor, pane) };
    const want = { lines: oracle(pane), cursor: oracleCursor(pane) };
    return JSON.stringify(got) === JSON.stringify(want) ? 'match' : JSON.stringify({ got, want });
  }, { timeout: 8000 }).toBe('match');
}

async function attach(page) {
  let queue = [];
  let flushing = false;
  const flush = async () => {
    if (flushing) return;
    flushing = true;
    while (queue.length) {
      const batch = queue;
      queue = [];
      await page.evaluate((frames) => window.__mockDeliver(frames), batch).catch(() => {});
    }
    flushing = false;
  };
  const conn = new ControlConnection(SESSION, (frame) => {
    queue.push(frame);
    flush();
  });
  await page.exposeFunction('__mockInvoke', async (cmd, args) => {
    switch (cmd) {
      case 'tmux_subscribe': return { origin: null, session: SESSION };
      case 'tmux_command': conn.write(args.commands, args.tags); return null;
      case 'tmux_send_keys': conn.sendKeys(args.pane, Buffer.from(args.data)); return null;
      case 'tmux_detach': conn.write(['detach-client'], [0]); return null;
      default: return null;
    }
  });
  await page.evaluate(() => window.__mockEmit('tmux-connection', { conn_id: 1, origin: null, session: 'e2e' }));
  await expect.poll(() => page.evaluate(() => {
    const panes = window.harness.panes();
    return panes.length > 0 && panes.every(p => p.live || !p.visible);
  }), { timeout: 8000 }).toBe(true);
  return conn;
}

let conn = null;

let serverSeq = 0;

test.beforeEach(async ({ page }) => {
  useSocket(`mdterm-e2e-${process.pid}-${serverSeq++}`);
  tmux('new-session', '-d', '-s', SESSION, '-x', '80', '-y', '24', 'env', 'PS1=$ ', 'sh');
  await page.goto('/tests/terminal/visual/tmux-harness.html');
  await page.waitForFunction(() => window.__harnessReady === true);
});

test.afterEach(async () => {
  if (conn) conn.kill();
  conn = null;
  killServer();
});

async function sendToTmux(keys) {
  tmux('send-keys', '-t', SESSION, keys, 'Enter');
  await new Promise(r => setTimeout(r, 300));
}

test('seeds existing pane content and tracks live output', async ({ page }) => {
  await sendToTmux('echo SEED-MARKER; printf "\\033[1;31mred\\033[0m ─│ 漢字\\n"');
  conn = await attach(page);
  let [pane] = await page.evaluate(() => window.harness.panes());
  // The client sizes tmux to the harness area (900x560px), not 80x24.
  await expect.poll(async () => {
    [pane] = await page.evaluate(() => window.harness.panes());
    return Number(tmux('display', '-p', '-t', `%${pane.id}`, '#{pane_width}').trim()) === pane.cols && pane.cols;
  }).toBeGreaterThan(80);
  await expectPaneMatchesTmux(page, pane.id);
  const lines = await screen(page, pane.id);
  expect(lines.filter(l => l === 'SEED-MARKER').length).toBe(1);

  await sendToTmux('seq 1 300');
  await expectPaneMatchesTmux(page, pane.id);
});

test('keyboard input reaches the pane', async ({ page }) => {
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), pane.id);
  await page.keyboard.type('echo typed-$((6*7))');
  await page.keyboard.press('Enter');
  await expect.poll(() => oracle(pane.id).includes('typed-42'), { timeout: 5000 }).toBe(true);
  await expectPaneMatchesTmux(page, pane.id);
});

test('splits render at tmux layout geometry and resize by dragging', async ({ page }) => {
  conn = await attach(page);
  const [first] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), first.id);
  await page.keyboard.press('Control+Shift+D');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.visible && p.live).length)).toBe(2);
  await page.keyboard.press('Control+Shift+E');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.visible && p.live).length)).toBe(3);

  const panes = await page.evaluate(() => window.harness.panes());
  for (const p of panes) {
    const [w, h] = tmux('display', '-p', '-t', `%${p.id}`, '#{pane_width} #{pane_height}').trim().split(' ').map(Number);
    expect([p.cols, p.rows]).toEqual([w, h]);
    await expectPaneMatchesTmux(page, p.id);
  }
  // No two panes overlap.
  for (const a of panes) {
    for (const b of panes) {
      if (a.id >= b.id) continue;
      const overlap = a.rect.x < b.rect.x + b.rect.w && b.rect.x < a.rect.x + a.rect.w &&
        a.rect.y < b.rect.y + b.rect.h && b.rect.y < a.rect.y + a.rect.h;
      expect(overlap).toBe(false);
    }
  }

  // Drag the vertical divider 10 columns to the left.
  const before = Number(tmux('display', '-p', '-t', `%${first.id}`, '#{pane_width}').trim());
  const divider = page.locator('.tmux-divider-v').first();
  const box = await divider.boundingBox();
  const cell = first.rect.w / first.cols;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 - cell * 10, box.y + box.height / 2, { steps: 5 });
  await page.mouse.up();
  await expect.poll(() => Number(tmux('display', '-p', '-t', `%${first.id}`, '#{pane_width}').trim())).toBe(before - 10);
  await expect.poll(() => page.evaluate((id) => window.harness.panes().find(p => p.id === id).cols, first.id)).toBe(before - 10);
});

test('tmux windows map to tabs', async ({ page }) => {
  conn = await attach(page);
  expect(await page.evaluate(() => window.harness.tabs().length)).toBe(1);
  await page.evaluate(() => window.harness.action('new-window'));
  await expect.poll(() => page.evaluate(() => window.harness.tabs().length)).toBe(2);
  // tmux made the new window current; the client followed.
  const tabs = await page.evaluate(() => window.harness.tabs());
  await expect.poll(() => page.evaluate(() => window.harness.active())).toBe(tabs[1]);
  await sendToTmux('echo IN-SECOND-WINDOW');
  const second = (await page.evaluate(() => window.harness.panes())).find(p => p.visible);
  await expectPaneMatchesTmux(page, second.id);

  await page.evaluate(() => window.harness.action('kill-window'));
  await expect.poll(() => page.evaluate(() => window.harness.tabs().length)).toBe(1);
});

test('seeds a pane that is in the alternate screen', async ({ page }) => {
  await sendToTmux('echo BEFORE-ALT');
  await sendToTmux("printf '\\033[?1049h\\033[H\\033[2JALT-SCREEN-CONTENT\\033[5;10HX'; sleep 60");
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  expect(pane.alt).toBe(true);
  await expectPaneMatchesTmux(page, pane.id);
  const normal = await page.evaluate((p) => window.harness.normalBuffer(p), pane.id);
  expect(normal.some(l => l.includes('BEFORE-ALT'))).toBe(true);
});

test('mdterm editor-open sequence works from inside a tmux pane', async ({ page }) => {
  conn = await attach(page);
  tmux('set', '-p', '-t', SESSION, 'allow-passthrough', 'on');
  const b64 = (v) => Buffer.from(v).toString('base64');
  // What bin/mdterm prints inside tmux: OSC 5337 wrapped in DCS passthrough.
  await sendToTmux(`printf '\\033Ptmux;\\033\\033]5337;open;${b64('notes.md')};${b64('/tmp/notes.md')};${b64('# hi')}\\007\\033\\\\'`);
  await expect.poll(() => page.evaluate(() => window.__openedFiles.length)).toBe(1);
  const opened = await page.evaluate(() => window.__openedFiles[0]);
  expect(opened).toMatchObject({ name: 'notes.md', path: '/tmp/notes.md', content: '# hi' });
  expect(opened.session_id).toMatch(/^tmux-1-\d+$/);
});

test('detach closes the tmux tabs', async ({ page }) => {
  conn = await attach(page);
  await page.evaluate(() => window.harness.action('detach'));
  await expect.poll(() => page.evaluate(() => window.__tmuxExited === true)).toBe(true);
  expect(await page.evaluate(() => window.harness.tabs().length)).toBe(0);
  // The session itself survives.
  expect(tmux('has-session', '-t', SESSION)).toBe('');
});
