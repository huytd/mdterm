// End-to-end tests for tmux control mode: the real frontend client
// (frontend/js/tmux-client.js) driven against a real tmux server on an
// isolated socket. tmux itself is the oracle for what each pane should show.
import { test, expect } from '@playwright/test';
import { ControlConnection, killServer, tmux, tmuxAvailable, useSocket } from './tmux-backend.mjs';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

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
  let box = null;
  await expect.poll(async () => {
    box = await divider.boundingBox();
    return box;
  }).not.toBeNull();
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

test('default prefix emulation runs bound commands without leaking keys', async ({ page }) => {
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), pane.id);

  // C-b % -> horizontal split (2 side-by-side panes)
  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  // Pane's screen does not contain a stray '%'
  expect(oracle(pane.id).some(l => l.includes('%'))).toBe(false);
  await expectPaneMatchesTmux(page, pane.id);

  // C-b " -> vertical split (3 panes)
  await page.keyboard.press('Control+b');
  await page.keyboard.press('"');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(3);

  // C-b z -> zoom active pane
  await page.keyboard.press('Control+b');
  await page.keyboard.press('z');
  await expect.poll(() => tmux('display', '-p', '#{window_zoomed_flag}').trim()).toBe('1');
  // Unzoom
  await page.keyboard.press('Control+b');
  await page.keyboard.press('z');
  await expect.poll(() => tmux('display', '-p', '#{window_zoomed_flag}').trim()).toBe('0');

  // C-b c -> new window (second tab appears)
  await page.keyboard.press('Control+b');
  await page.keyboard.press('c');
  await expect.poll(() => page.evaluate(() => window.harness.tabs().length)).toBe(2);
  const tabs = await page.evaluate(() => window.harness.tabs());
  await expect.poll(() => page.evaluate(() => window.harness.active())).toBe(tabs[1]);

  // C-b p / C-b n -> tab switching
  await page.keyboard.press('Control+b');
  await page.keyboard.press('p');
  await expect.poll(() => page.evaluate(() => window.harness.active())).toBe(tabs[0]);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('n');
  await expect.poll(() => page.evaluate(() => window.harness.active())).toBe(tabs[1]);
});

test('C-b C-b sends prefix key to the active pane', async ({ page }) => {
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), pane.id);

  await sendToTmux('cat -v');
  await page.keyboard.press('Control+b');
  await page.keyboard.press('Control+b');
  await expect.poll(() => oracle(pane.id).some(l => l.includes('^B')), { timeout: 5000 }).toBe(true);
  await page.keyboard.press('Control+c');
});

test('custom tmux config with prefix, custom bindings, repeat, and root bindings', async ({ page }) => {
  if (conn) conn.kill();
  killServer();

  const confPath = path.join(os.tmpdir(), `mdterm-custom-${process.pid}-${Date.now()}.conf`);
  fs.writeFileSync(confPath, [
    'set -g prefix C-a',
    'bind | split-window -h',
    'bind -r H resize-pane -L 5',
    'bind -n M-h select-pane -L',
    ''
  ].join('\n'));

  try {
    useSocket(`mdterm-e2e-${process.pid}-${serverSeq++}`, confPath);
    tmux('new-session', '-d', '-s', SESSION, '-x', '80', '-y', '24', 'env', 'PS1=$ ', 'sh');
    await page.reload();
    await page.waitForFunction(() => window.__harnessReady === true);

    conn = await attach(page);
    const [leftPane] = await page.evaluate(() => window.harness.panes());
    await page.evaluate((p) => window.harness.focusPane(p), leftPane.id);

    // C-a | -> horizontal split
    await page.keyboard.press('Control+a');
    await page.keyboard.press('|');
    await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

    const panes = await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible));
    const rightPane = panes.find(p => p.id !== leftPane.id);
    await page.evaluate((p) => window.harness.focusPane(p), rightPane.id);

    const w0 = Number(tmux('display', '-p', '-t', `%${leftPane.id}`, '#{pane_width}').trim());

    // C-a H H -> repeat resize left 5 twice = 10 cols
    await page.keyboard.press('Control+a');
    await page.keyboard.press('H');
    await page.keyboard.press('H');
    await expect.poll(() => Number(tmux('display', '-p', '-t', `%${leftPane.id}`, '#{pane_width}').trim())).toBe(w0 - 10);

    // Wait repeat-time (500ms) + 150ms, press H -> passes to pane
    await page.waitForTimeout(650);
    await page.keyboard.press('H');
    await page.keyboard.press('Enter');
    await expect.poll(() => oracle(rightPane.id).some(l => l.includes('H')), { timeout: 5000 }).toBe(true);

    // Root binding M-h moves focus left
    await page.keyboard.press('Alt+h');
    await expect.poll(() => tmux('display', '-p', '#{pane_id}').trim()).toBe(`%${leftPane.id}`);
  } finally {
    fs.rmSync(confPath, { force: true });
  }
});

test('native UI: command-prompt, window rename, confirm-before, copy-mode, detach', async ({ page }) => {
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), pane.id);

  // C-b : -> prompt overlay visible
  await page.keyboard.press('Control+b');
  await page.keyboard.press(':');
  const promptLocator = page.locator('.tmux-overlay-prompt');
  await expect(promptLocator).toBeVisible();

  // Type split-window -v, Enter -> split happens
  await page.keyboard.type('split-window -v');
  await page.keyboard.press('Enter');
  await expect(promptLocator).toBeHidden();
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  // C-b , -> prompt prefilled with window name
  await page.keyboard.press('Control+b');
  await page.keyboard.press(',');
  await expect(promptLocator).toBeVisible();
  const input = page.locator('.tmux-prompt-input');
  await input.fill('renamed-tab');
  await page.keyboard.press('Enter');
  await expect(promptLocator).toBeHidden();
  await expect.poll(() => page.evaluate(() => (window.__renamedWindows || []).some(w => w.name === 'renamed-tab'))).toBe(true);

  // C-b x -> confirm overlay
  await page.keyboard.press('Control+b');
  await page.keyboard.press('x');
  const confirmLocator = page.locator('.tmux-overlay-confirm');
  await expect(confirmLocator).toBeVisible();

  // Press n -> cancel (still 2 panes)
  await page.keyboard.press('n');
  await expect(confirmLocator).toBeHidden();
  expect(await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  // Press C-b x, then y -> pane killed (1 pane)
  await page.keyboard.press('Control+b');
  await page.keyboard.press('x');
  await expect(confirmLocator).toBeVisible();
  await page.keyboard.press('y');
  await expect(confirmLocator).toBeHidden();
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(1);

  // C-b [ -> scrollback badge shown, pane not in copy mode
  await page.keyboard.press('Control+b');
  await page.keyboard.press('[');
  const badgeLocator = page.locator('.tmux-scrollback-badge');
  await expect(badgeLocator).toBeVisible();
  const mode = tmux('display', '-p', '-t', `%${pane.id}`, '#{pane_mode}').trim();
  expect(mode).toBe('');

  // C-b d -> detach
  await page.keyboard.press('Control+b');
  await page.keyboard.press('d');
  await expect.poll(() => page.evaluate(() => window.__tmuxExited === true)).toBe(true);
});

test('safety net: external copy-mode shows overlay and Esc cancels', async ({ page }) => {
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), pane.id);

  // Enter copy-mode from outside tmux
  tmux('copy-mode', '-t', `%${pane.id}`);
  const overlay = page.locator('.tmux-mode-overlay');
  await expect(overlay).toBeVisible();

  // Press Esc -> cancels copy mode
  await page.keyboard.press('Escape');
  await expect(overlay).toBeHidden();
  await expect.poll(() => tmux('display', '-p', '-t', `%${pane.id}`, '#{pane_mode}').trim()).toBe('');
});


test('prompt input is passed to tmux literally', async ({ page }) => {
  conn = await attach(page);
  const [pane] = await page.evaluate(() => window.harness.panes());
  await page.evaluate((p) => window.harness.focusPane(p), pane.id);
  const hostile = `a'b "q" $HOME \\ ; kill-server`;
  await page.keyboard.press('Control+b');
  await page.keyboard.press(',');
  await page.locator('.tmux-prompt-input').fill(hostile);
  await page.keyboard.press('Enter');
  // tmux stores window names with backslashes escaped (\ -> \\), as its own prompt would.
  await expect.poll(() => tmux('display', '-p', '#{window_name}').replace(/\n$/, '')).toBe(hostile.replace(/\\/g, '\\\\'));
  expect(tmux('has-session', '-t', SESSION)).toBe('');
});

test('drag pane to centre swaps panes and selects dragged pane', async ({ page }) => {
  conn = await attach(page);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  const panes = await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible));
  panes.sort((a, b) => a.rect.x - b.rect.x);
  const leftPane = panes[0];
  const rightPane = panes[1];

  tmux('send-keys', '-t', `%${leftPane.id}`, 'echo LEFT-PANE', 'Enter');
  tmux('send-keys', '-t', `%${rightPane.id}`, 'echo RIGHT-PANE', 'Enter');
  await expectPaneMatchesTmux(page, leftPane.id);
  await expectPaneMatchesTmux(page, rightPane.id);

  const leftGrip = page.locator(`.tmux-pane[data-pane="${leftPane.id}"] .tmux-pane-grip`);
  await expect(leftGrip).toBeAttached();
  const gripBox = await leftGrip.boundingBox();
  expect(gripBox).not.toBeNull();

  const rightBox = rightPane.rect;
  const targetX = rightBox.x + rightBox.w / 2;
  const targetY = rightBox.y + rightBox.h / 2;

  await page.mouse.move(gripBox.x + gripBox.width / 2, gripBox.y + gripBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(targetX, targetY, { steps: 5 });
  await page.mouse.up();

  await expect.poll(() => {
    return tmux('list-panes', '-F', '#{pane_id} #{pane_left}').trim().split('\n');
  }).toEqual(expect.arrayContaining([
    expect.stringMatching(new RegExp(`%${leftPane.id}\\s+[1-9]\\d*`)),
    expect.stringMatching(new RegExp(`%${rightPane.id}\\s+0`))
  ]));

  await expectPaneMatchesTmux(page, leftPane.id);
  await expectPaneMatchesTmux(page, rightPane.id);
  await expect.poll(() => tmux('display', '-p', '#{pane_id}').trim()).toBe(`%${leftPane.id}`);
});

test('drag pane to bottom edge moves pane below target', async ({ page }) => {
  conn = await attach(page);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  const panes = await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible));
  panes.sort((a, b) => a.rect.x - b.rect.x);
  const leftPane = panes[0];
  const rightPane = panes[1];

  tmux('send-keys', '-t', `%${leftPane.id}`, 'echo TOP-OR-LEFT', 'Enter');
  tmux('send-keys', '-t', `%${rightPane.id}`, 'echo BOTTOM-OR-RIGHT', 'Enter');
  await expectPaneMatchesTmux(page, leftPane.id);
  await expectPaneMatchesTmux(page, rightPane.id);

  const leftGrip = page.locator(`.tmux-pane[data-pane="${leftPane.id}"] .tmux-pane-grip`);
  const gripBox = await leftGrip.boundingBox();
  expect(gripBox).not.toBeNull();

  const rightBox = rightPane.rect;
  const targetX = rightBox.x + rightBox.w / 2;
  const targetY = rightBox.y + rightBox.h - 10;

  await page.mouse.move(gripBox.x + gripBox.width / 2, gripBox.y + gripBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(targetX, targetY, { steps: 5 });
  await page.mouse.up();

  await expect.poll(() => tmux('display', '-p', '#{window_layout}').trim()).toContain('[');
  await expectPaneMatchesTmux(page, leftPane.id);
  await expectPaneMatchesTmux(page, rightPane.id);
});

test('drag cancel via Escape leaves layout unchanged', async ({ page }) => {
  conn = await attach(page);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  const layoutBefore = tmux('display', '-p', '#{window_layout}').trim();
  const [leftPane, rightPane] = (await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible))).sort((a, b) => a.rect.x - b.rect.x);

  const leftGrip = page.locator(`.tmux-pane[data-pane="${leftPane.id}"] .tmux-pane-grip`);
  const gripBox = await leftGrip.boundingBox();
  expect(gripBox).not.toBeNull();

  await page.mouse.move(gripBox.x + gripBox.width / 2, gripBox.y + gripBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(rightPane.rect.x + rightPane.rect.w / 2, rightPane.rect.y + rightPane.rect.h / 2, { steps: 5 });
  await expect(page.locator('.tmux-drag-ghost')).toBeAttached();

  await page.keyboard.press('Escape');
  await page.mouse.up();

  expect(tmux('display', '-p', '#{window_layout}').trim()).toBe(layoutBefore);
  await expect(page.locator('.tmux-drag-ghost')).toHaveCount(0);
});

test('drop pane on self is a no-op', async ({ page }) => {
  conn = await attach(page);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  const layoutBefore = tmux('display', '-p', '#{window_layout}').trim();
  const [leftPane] = (await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible))).sort((a, b) => a.rect.x - b.rect.x);

  const leftGrip = page.locator(`.tmux-pane[data-pane="${leftPane.id}"] .tmux-pane-grip`);
  const gripBox = await leftGrip.boundingBox();
  expect(gripBox).not.toBeNull();

  await page.mouse.move(gripBox.x + gripBox.width / 2, gripBox.y + gripBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(leftPane.rect.x + leftPane.rect.w / 2, leftPane.rect.y + leftPane.rect.h / 2, { steps: 5 });
  await page.mouse.up();

  expect(tmux('display', '-p', '#{window_layout}').trim()).toBe(layoutBefore);
  await expect(page.locator('.tmux-drag-ghost')).toHaveCount(0);
});

test('single-pane and zoomed window hide grip and ignore Alt+Shift drag', async ({ page }) => {
  conn = await attach(page);
  const [singlePane] = await page.evaluate(() => window.harness.panes());

  await expect(page.locator('.tmux-pane-grip:visible')).toHaveCount(0);

  const layoutBefore = tmux('display', '-p', '#{window_layout}').trim();
  await page.keyboard.down('Alt');
  await page.keyboard.down('Shift');
  await page.mouse.move(singlePane.rect.x + 50, singlePane.rect.y + 50);
  await page.mouse.down();
  await page.mouse.move(singlePane.rect.x + 150, singlePane.rect.y + 150, { steps: 5 });
  await page.mouse.up();
  await page.keyboard.up('Shift');
  await page.keyboard.up('Alt');

  expect(tmux('display', '-p', '#{window_layout}').trim()).toBe(layoutBefore);
  await expect(page.locator('.tmux-drag-ghost')).toHaveCount(0);

  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  await page.keyboard.press('Control+b');
  await page.keyboard.press('z');
  await expect.poll(() => tmux('display', '-p', '#{window_zoomed_flag}').trim()).toBe('1');

  await expect(page.locator('.tmux-pane-grip:visible')).toHaveCount(0);

  await page.keyboard.down('Alt');
  await page.keyboard.down('Shift');
  await page.mouse.move(singlePane.rect.x + 50, singlePane.rect.y + 50);
  await page.mouse.down();
  await page.mouse.move(singlePane.rect.x + 150, singlePane.rect.y + 150, { steps: 5 });
  await page.mouse.up();
  await page.keyboard.up('Shift');
  await page.keyboard.up('Alt');

  expect(tmux('display', '-p', '#{window_zoomed_flag}').trim()).toBe('1');
  await expect(page.locator('.tmux-drag-ghost')).toHaveCount(0);

  await page.keyboard.press('Control+b');
  await page.keyboard.press('z');
  await expect.poll(() => tmux('display', '-p', '#{window_zoomed_flag}').trim()).toBe('0');
  await expect(page.locator('.tmux-pane-grip').first()).toBeAttached();
});

test('plain drag inside a pane selects text without moving panes', async ({ page }) => {
  conn = await attach(page);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('%');
  await expect.poll(() => page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible).length)).toBe(2);

  const [pane] = (await page.evaluate(() => window.harness.panes().filter(p => p.live && p.visible))).sort((a, b) => a.rect.x - b.rect.x);
  tmux('send-keys', '-t', `%${pane.id}`, 'echo SELECTION_TEST_MARKER', 'Enter');
  await expectPaneMatchesTmux(page, pane.id);

  const layoutBefore = tmux('display', '-p', '#{window_layout}').trim();

  const startX = pane.rect.x + 20;
  const startY = pane.rect.y + 60;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + 180, startY, { steps: 5 });
  await page.mouse.up();

  await expect.poll(() => page.evaluate((p) => window.harness.hasSelection(p), pane.id)).toBe(true);

  expect(tmux('display', '-p', '#{window_layout}').trim()).toBe(layoutBefore);
  await expect(page.locator('.tmux-drag-ghost')).toHaveCount(0);
});
