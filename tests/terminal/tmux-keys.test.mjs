import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import {
  keyEventToTmux,
  parseListKeys,
  tokenize,
  classifyCommand,
  createKeyRouter
} from '../../frontend/js/tmux-keys.js';

test('keyEventToTmux maps events correctly', () => {
  // C-b
  assert.equal(keyEventToTmux({ key: 'b', ctrlKey: true }), 'C-b');
  // Printable char: %
  assert.equal(keyEventToTmux({ key: '%', shiftKey: true }), '%');
  // "
  assert.equal(keyEventToTmux({ key: '"', shiftKey: true }), '"');
  // C (shift)
  assert.equal(keyEventToTmux({ key: 'C', shiftKey: true }), 'C');
  // Space
  assert.equal(keyEventToTmux({ key: ' ' }), 'Space');
  // Up
  assert.equal(keyEventToTmux({ key: 'ArrowUp' }), 'Up');
  // S-Up
  assert.equal(keyEventToTmux({ key: 'ArrowUp', shiftKey: true }), 'S-Up');
  // C-Up
  assert.equal(keyEventToTmux({ key: 'ArrowUp', ctrlKey: true }), 'C-Up');
  // M-1
  assert.equal(keyEventToTmux({ key: '1', altKey: true }), 'M-1');
  // C-M-Up
  assert.equal(keyEventToTmux({ key: 'ArrowUp', ctrlKey: true, altKey: true }), 'C-M-Up');
  // PPage
  assert.equal(keyEventToTmux({ key: 'PageUp' }), 'PPage');
  // DC
  assert.equal(keyEventToTmux({ key: 'Delete' }), 'DC');
  // BTab
  assert.equal(keyEventToTmux({ key: 'Tab', shiftKey: true }), 'BTab');
  // Escape
  assert.equal(keyEventToTmux({ key: 'Escape' }), 'Escape');

  // metaKey -> null
  assert.equal(keyEventToTmux({ key: 'b', ctrlKey: true, metaKey: true }), null);
  assert.equal(keyEventToTmux({ key: 'x', metaKey: true }), null);

  // modifier-only -> null
  assert.equal(keyEventToTmux({ key: 'Shift' }), null);
  assert.equal(keyEventToTmux({ key: 'Control' }), null);
  assert.equal(keyEventToTmux({ key: 'Alt' }), null);

  // Ctrl+Shift+T -> null
  assert.equal(keyEventToTmux({ key: 'T', ctrlKey: true, shiftKey: true }), null);
  assert.equal(keyEventToTmux({ key: 'w', code: 'KeyW', ctrlKey: true, shiftKey: true }), null);
});

test('tokenize handles quotes, braces, escaped \\;, and top-level ;', () => {
  // quotes
  const qTokens = tokenize('echo "hello world" \'another test\'');
  assert.deepEqual(qTokens, ['echo', 'hello world', 'another test']);

  // braces kept intact as one token
  const bTokens = tokenize('command-prompt -I "#W" { rename-window "%%" }');
  assert.deepEqual(bTokens, ['command-prompt', '-I', '#W', '{ rename-window "%%" }']);

  // escaped \; inside or between args
  const escTokens = tokenize('echo a\\;b ; echo c');
  assert.deepEqual(escTokens, ['echo', 'a;b', ';', 'echo', 'c']);

  // top-level ; and standalone \; splitting
  const splitTokens = tokenize('split-window -h ; select-pane -L');
  assert.deepEqual(splitTokens, ['split-window', '-h', ';', 'select-pane', '-L']);

  const splitTokens2 = tokenize('split-window -h \\; select-pane -L');
  assert.deepEqual(splitTokens2, ['split-window', '-h', ';', 'select-pane', '-L']);
});

test('parseListKeys parses lines and unescapes keys', () => {
  let lines;
  try {
    lines = execFileSync('tmux', ['-f', '/dev/null', 'list-keys'], { encoding: 'utf8' }).trim().split('\n');
  } catch {
    lines = [
      'bind-key    -T prefix       \\%                        split-window -h',
      'bind-key    -T prefix       \\"                        split-window',
      'bind-key -r -T prefix       Up                        select-pane -U',
      'bind-key -r -T prefix       M-Up                      resize-pane -U 5',
      'bind-key -r -T prefix       C-Up                      resize-pane -U',
      'bind-key    -T prefix       c                         new-window',
      'bind-key    -T root         DoubleClick1Pane          select-pane -t = \\; if-shell -F "#{||:#{pane_in_mode},#{mouse_any_flag}}" { send-keys -M } { copy-mode -H ; send-keys -X select-word ; run-shell -d 0.3 ; send-keys -X copy-pipe-and-cancel }'
    ];
  }

  const { tables } = parseListKeys(lines);
  assert.ok(tables.prefix);
  assert.ok(tables.prefix.size > 5);

  // \% -> %
  const pctBinding = tables.prefix.get('%');
  assert.ok(pctBinding, 'found % binding');
  assert.ok(pctBinding.command.startsWith('split-window -h'));

  // -r flags on Up/M-Up/C-Up
  const upBinding = tables.prefix.get('Up');
  assert.ok(upBinding && upBinding.repeat, 'Up is repeatable');
  const mUpBinding = tables.prefix.get('M-Up');
  assert.ok(mUpBinding && mUpBinding.repeat, 'M-Up is repeatable');
  const cUpBinding = tables.prefix.get('C-Up');
  assert.ok(cUpBinding && cUpBinding.repeat, 'C-Up is repeatable');

  // {…} commands kept intact
  const doubleClick = tables.root?.get('DoubleClick1Pane');
  if (doubleClick) {
    assert.ok(doubleClick.command.includes('{ send-keys -M }'));
  }
});

test('classifyCommand classifies run vs native commands', () => {
  // split-window -h → run
  const runRes = classifyCommand('split-window -h');
  assert.equal(runRes.kind, 'run');

  // copy-mode → native
  const copyRes = classifyCommand('copy-mode');
  assert.equal(copyRes.kind, 'native');
  assert.equal(copyRes.name, 'copy-mode');
  assert.equal(copyRes.scrollUp, false);

  const copyURes = classifyCommand('copy-mode -u');
  assert.equal(copyURes.kind, 'native');
  assert.equal(copyURes.name, 'copy-mode');
  assert.equal(copyURes.scrollUp, true);

  // confirm-before -p "kill-pane #P? (y/n)" kill-pane → native confirm with prompt+command
  const confRes = classifyCommand('confirm-before -p "kill-pane #P? (y/n)" kill-pane');
  assert.equal(confRes.kind, 'native');
  assert.equal(confRes.name, 'confirm-before');
  assert.equal(confRes.prompt, 'kill-pane #P? (y/n)');
  assert.equal(confRes.cmd, 'kill-pane');

  // command-prompt -I "#W" { rename-window "%%" } → native prompt with initial #W and template
  const promptRes = classifyCommand('command-prompt -I "#W" { rename-window "%%" }');
  assert.equal(promptRes.kind, 'native');
  assert.equal(promptRes.name, 'command-prompt');
  assert.equal(promptRes.initial, '#W');
  assert.equal(promptRes.template, '{ rename-window "%%" }');

  // if-shell -F 1 { copy-mode } → native (UI command nested)
  const nestedRes = classifyCommand('if-shell -F 1 { copy-mode }');
  assert.equal(nestedRes.kind, 'native');
  assert.equal(nestedRes.name, 'copy-mode');

  // detach-client -> native
  const detachRes = classifyCommand('detach-client');
  assert.equal(detachRes.kind, 'native');
  assert.equal(detachRes.name, 'detach-client');

  // unsupported command -> native unsupported
  const menuRes = classifyCommand('display-menu -T menu');
  assert.equal(menuRes.kind, 'native');
  assert.equal(menuRes.name, 'unsupported');
});

test('createKeyRouter routing and state machine', () => {
  let currentTime = 1000;
  const now = () => currentTime;

  const prefixTable = new Map([
    ['%', { command: 'split-window -h', repeat: false }],
    ['"', { command: 'split-window -v', repeat: false }],
    ['Up', { command: 'resize-pane -U', repeat: true }],
    ['m', { command: 'switch-client -T mytable', repeat: false }],
    ['C-b', { command: 'send-prefix', repeat: false }]
  ]);

  const rootTable = new Map([
    ['M-h', { command: 'select-pane -L', repeat: false }]
  ]);

  const myTable = new Map([
    ['x', { command: 'split-window -v', repeat: false }]
  ]);

  const tables = {
    prefix: prefixTable,
    root: rootTable,
    mytable: myTable
  };

  const router = createKeyRouter({
    tables,
    prefix: 'C-b',
    prefix2: 'C-a',
    repeatTime: 500,
    now
  });

  // Prefix then % → run split-window -h, back to root
  assert.deepEqual(router.handle('C-b'), { consume: true });
  assert.equal(router.state().table, 'prefix');
  assert.deepEqual(router.handle('%'), { consume: true, run: 'split-window -h' });
  assert.equal(router.state().table, 'root');

  // Unbound key after prefix → consumed, root
  assert.deepEqual(router.handle('C-b'), { consume: true });
  assert.deepEqual(router.handle('k'), { consume: true });
  assert.equal(router.state().table, 'root');

  // -r binding repeat within repeatTime runs again without prefix
  assert.deepEqual(router.handle('C-b'), { consume: true });
  assert.deepEqual(router.handle('Up'), { consume: true, run: 'resize-pane -U' });
  assert.equal(router.state().table, 'prefix');
  assert.equal(router.state().inRepeat, true);

  currentTime += 200; // 1200 <= 1500
  assert.deepEqual(router.handle('Up'), { consume: true, run: 'resize-pane -U' });
  assert.equal(router.state().table, 'prefix');

  // After expiry the key passes through (consume: false)
  currentTime += 600; // 1800 > 1700
  assert.deepEqual(router.handle('a'), { consume: false });
  assert.equal(router.state().table, 'root');

  // switch-client -T mytable then key looks up mytable
  assert.deepEqual(router.handle('C-b'), { consume: true });
  assert.deepEqual(router.handle('m'), { consume: true });
  assert.equal(router.state().table, 'mytable');
  assert.deepEqual(router.handle('x'), { consume: true, run: 'split-window -v' });
  assert.equal(router.state().table, 'root');

  // prefix2
  assert.deepEqual(router.handle('C-a'), { consume: true });
  assert.equal(router.state().table, 'prefix');
  assert.deepEqual(router.handle('%'), { consume: true, run: 'split-window -h' });
  assert.equal(router.state().table, 'root');

  // root binding M-h
  assert.deepEqual(router.handle('M-h'), { consume: true, run: 'select-pane -L' });
  assert.equal(router.state().table, 'root');

  // C-b C-b → send-prefix
  assert.deepEqual(router.handle('C-b'), { consume: true });
  assert.deepEqual(router.handle('C-b'), { consume: true, run: 'send-prefix', sendPrefix: true });
  assert.equal(router.state().table, 'root');
});
