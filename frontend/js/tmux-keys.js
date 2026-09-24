/**
 * tmux-keys.js — pure, DOM-free key event translation, table parsing, and routing.
 */

const ALIASES = {
  splitw: 'split-window',
  neww: 'new-window',
  selectp: 'select-pane',
  selectw: 'select-window',
  killp: 'kill-pane',
  killw: 'kill-window',
  resizep: 'resize-pane',
  swapp: 'swap-pane',
  breakp: 'break-pane',
  joinp: 'join-pane',
  movep: 'move-pane',
  detach: 'detach-client',
  display: 'display-message',
  displayp: 'display-panes',
  confirm: 'confirm-before',
  menu: 'display-menu',
  popup: 'display-popup',
  lsk: 'list-keys',
  lsb: 'list-buffers',
  showmsgs: 'show-messages',
  findw: 'find-window',
  switchc: 'switch-client',
  suspendc: 'suspend-client'
};

const UNSUPPORTED_COMMANDS = new Set([
  'display-menu',
  'display-popup',
  'customize-mode',
  'clock-mode',
  'choose-buffer',
  'choose-client',
  'show-messages',
  'list-keys',
  'find-window',
  'suspend-client',
  'choose-tree',
  'choose-session',
  'choose-window',
  'display-panes'
]);

const NAMED_KEYS = {
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Enter: 'Enter',
  Escape: 'Escape',
  Backspace: 'BSpace',
  Delete: 'DC',
  Insert: 'IC',
  Home: 'Home',
  End: 'End',
  PageUp: 'PPage',
  PageDown: 'NPage',
  ' ': 'Space',
  Space: 'Space',
  Spacebar: 'Space'
};

for (let i = 1; i <= 12; i++) {
  NAMED_KEYS[`F${i}`] = `F${i}`;
}

/**
 * KeyboardEvent-like {key, code, ctrlKey, altKey, shiftKey, metaKey, isComposing} → tmux key name or null.
 */
export function keyEventToTmux(ev) {
  if (!ev) return null;
  if (ev.isComposing) return null;

  const key = ev.key;
  if (!key) return null;

  // Ignore pure modifier presses
  if (['Shift', 'Control', 'Alt', 'Meta', 'AltGraph', 'CapsLock', 'NumLock', 'ScrollLock'].includes(key)) {
    return null;
  }

  // Super / Cmd reserved for mdterm shortcuts
  if (ev.metaKey) return null;

  // Ctrl+Shift+<letter> reserved for mdterm shortcuts (Ctrl+Shift+T, W, D, E, etc.)
  const isLetter = /^[a-zA-Z]$/.test(key) || /^Key[A-Za-z]$/i.test(ev.code || '');
  if (ev.ctrlKey && ev.shiftKey && isLetter) {
    return null;
  }

  // Handle Tab / Backtab (Shift+Tab in browsers can be Tab with shiftKey or Backtab)
  if (key === 'Tab' || key === 'Backtab') {
    const isBacktab = ev.shiftKey || key === 'Backtab';
    const base = isBacktab ? 'BTab' : 'Tab';
    let mod = '';
    if (ev.ctrlKey) mod += 'C-';
    if (ev.altKey) mod += 'M-';
    return mod + base;
  }

  // Named keys
  const named = NAMED_KEYS[key];
  if (named) {
    let mod = '';
    if (ev.ctrlKey) mod += 'C-';
    if (ev.altKey) mod += 'M-';
    if (ev.shiftKey) mod += 'S-';
    return mod + named;
  }

  // Fallback for letter code (e.g. non-US keyboards or Option symbol on macOS)
  let codeLetter = null;
  if (ev.code && /^Key([A-Za-z])$/i.test(ev.code)) {
    codeLetter = RegExp.$1;
  }

  // Ctrl + letter or Ctrl + printable
  if (ev.ctrlKey) {
    let baseChar = null;
    if (codeLetter) {
      baseChar = codeLetter.toLowerCase();
    } else if (/^[a-zA-Z]$/.test(key)) {
      baseChar = key.toLowerCase();
    } else if (key === ' ' || key === 'Space') {
      baseChar = 'Space';
    } else if (key.length === 1) {
      baseChar = key;
    }

    if (baseChar) {
      let mod = 'C-';
      if (ev.altKey) mod = 'C-M-';
      return mod + baseChar;
    }
  }

  // Alt + printable (M-)
  if (ev.altKey) {
    let baseChar = null;
    if (codeLetter) {
      baseChar = ev.shiftKey ? codeLetter.toUpperCase() : codeLetter.toLowerCase();
    } else if (ev.code && /^Digit([0-9])$/.test(ev.code)) {
      baseChar = RegExp.$1;
    } else if (key.length === 1) {
      baseChar = key;
    }

    if (baseChar) {
      return 'M-' + baseChar;
    }
  }

  // Printable single character (Shift is already folded in, do not add S-)
  if (key.length === 1) {
    return key;
  }

  return null;
}

/**
 * Minimal tmux command tokenizer: words, "double", 'single', {braces} (kept as one token), \escapes, top-level ';'
 */
export function tokenize(commandText) {
  if (!commandText || typeof commandText !== 'string') return [];
  const tokens = [];
  let i = 0;
  const n = commandText.length;

  while (i < n) {
    // Skip whitespace
    while (i < n && /\s/.test(commandText[i])) i++;
    if (i >= n) break;

    const ch = commandText[i];

    // Top-level semicolon
    if (ch === ';') {
      tokens.push(';');
      i++;
      continue;
    }

    // Standalone escaped semicolon \;
    if (ch === '\\' && i + 1 < n && commandText[i + 1] === ';') {
      const beforeIsSpace = i === 0 || /\s/.test(commandText[i - 1]);
      const afterIsSpace = i + 2 >= n || /\s/.test(commandText[i + 2]);
      if (beforeIsSpace && afterIsSpace) {
        tokens.push(';');
        i += 2;
        continue;
      }
    }

    // Double quote
    if (ch === '"') {
      i++;
      let str = '';
      while (i < n && commandText[i] !== '"') {
        if (commandText[i] === '\\' && i + 1 < n) {
          str += commandText[i + 1];
          i += 2;
        } else {
          str += commandText[i];
          i++;
        }
      }
      if (i < n && commandText[i] === '"') i++;
      tokens.push(str);
      continue;
    }

    // Single quote
    if (ch === "'") {
      i++;
      let str = '';
      while (i < n && commandText[i] !== "'") {
        str += commandText[i];
        i++;
      }
      if (i < n && commandText[i] === "'") i++;
      tokens.push(str);
      continue;
    }

    // Brace block { ... } - kept as one token with outer braces
    if (ch === '{') {
      let depth = 0;
      let block = '';
      let inDQuote = false;
      let inSQuote = false;
      while (i < n) {
        const c = commandText[i];
        if (inDQuote) {
          block += c;
          if (c === '\\' && i + 1 < n) {
            block += commandText[++i];
          } else if (c === '"') {
            inDQuote = false;
          }
        } else if (inSQuote) {
          block += c;
          if (c === "'") {
            inSQuote = false;
          }
        } else {
          if (c === '"') inDQuote = true;
          else if (c === "'") inSQuote = true;
          else if (c === '{') depth++;
          else if (c === '}') {
            depth--;
            block += c;
            i++;
            if (depth === 0) break;
            continue;
          }
          block += c;
        }
        i++;
      }
      tokens.push(block);
      continue;
    }

    // Normal word / argument (can contain escaped chars e.g. \ )
    let word = '';
    while (i < n && !/\s/.test(commandText[i]) && commandText[i] !== ';' && commandText[i] !== '{' && commandText[i] !== '"' && commandText[i] !== "'") {
      if (commandText[i] === '\\' && i + 1 < n && commandText[i + 1] === ';') {
        const afterIsSpace = i + 2 >= n || /\s/.test(commandText[i + 2]);
        if (word === '' && afterIsSpace) {
          break;
        }
      }

      if (commandText[i] === '\\' && i + 1 < n) {
        word += commandText[i + 1];
        i += 2;
      } else {
        word += commandText[i];
        i++;
      }
    }
    if (word.length > 0) {
      tokens.push(word);
    }
  }

  return tokens;
}

/**
 * list-keys output lines → { tables: { [table]: Map<keyName, { command, repeat }> } }
 */
export function parseListKeys(lines) {
  const tables = {};
  if (!lines || !Array.isArray(lines)) return { tables };

  for (const line of lines) {
    if (!line || typeof line !== 'string') continue;
    const trimmed = line.trim();
    if (!trimmed.startsWith('bind-key')) continue;

    // Pattern: bind-key [-r] [-N "note"] -T <table> <key> <command...>
    const m = trimmed.match(/^bind-key\s+(?:.*?\s+)?-T\s+(\S+)\s+(\S+)\s+(.*)$/);
    if (!m) continue;

    const repeat = /(?:^|\s)-r(?:\s|$)/.test(trimmed.slice(0, trimmed.indexOf('-T')));
    const table = m[1];
    let key = m[2];
    const command = m[3].trim();

    if (key.length === 2 && key[0] === '\\') {
      key = key[1];
    }

    if (!tables[table]) {
      tables[table] = new Map();
    }
    tables[table].set(key, { command, repeat });
  }

  return { tables };
}

/**
 * Splits token array on top-level ';' into commands.
 */
function splitCommands(tokens) {
  const cmds = [];
  let curr = [];
  for (const t of tokens) {
    if (t === ';') {
      if (curr.length) cmds.push(curr);
      curr = [];
    } else {
      curr.push(t);
    }
  }
  if (curr.length) cmds.push(curr);
  return cmds;
}

/**
 * Command text → { kind: 'run' } | { kind: 'native', name, ... }
 */
export function classifyCommand(commandText) {
  const tokens = tokenize(commandText);
  const cmds = splitCommands(tokens);

  for (const cmdTokens of cmds) {
    if (cmdTokens.length === 0) continue;
    const rawName = cmdTokens[0];
    const name = ALIASES[rawName] || rawName;

    // copy-mode
    if (name === 'copy-mode') {
      return {
        kind: 'native',
        name: 'copy-mode',
        scrollUp: cmdTokens.includes('-u'),
        args: cmdTokens.slice(1)
      };
    }

    // confirm-before
    if (name === 'confirm-before') {
      let prompt = null;
      let remaining = [];
      for (let i = 1; i < cmdTokens.length; i++) {
        if (cmdTokens[i] === '-p' && i + 1 < cmdTokens.length) {
          prompt = cmdTokens[i + 1];
          i++;
        } else {
          remaining.push(cmdTokens[i]);
        }
      }
      const cmd = remaining.join(' ');
      if (!prompt) {
        prompt = `Confirm '${cmd}'? (y/n)`;
      }
      return {
        kind: 'native',
        name: 'confirm-before',
        prompt,
        cmd,
        command: cmd,
        args: cmdTokens.slice(1)
      };
    }

    // command-prompt
    if (name === 'command-prompt') {
      let initial = '';
      let prompt = ':';
      let template = '';
      const nonFlag = [];

      for (let i = 1; i < cmdTokens.length; i++) {
        if (cmdTokens[i] === '-I' && i + 1 < cmdTokens.length) {
          initial = cmdTokens[i + 1];
          i++;
        } else if (cmdTokens[i] === '-p' && i + 1 < cmdTokens.length) {
          prompt = cmdTokens[i + 1];
          i++;
        } else if (cmdTokens[i].startsWith('-')) {
          // Other flags (e.g. -F)
          continue;
        } else {
          nonFlag.push(cmdTokens[i]);
        }
      }

      if (nonFlag.length > 0) {
        template = nonFlag[nonFlag.length - 1];
      }

      return {
        kind: 'native',
        name: 'command-prompt',
        initial,
        prompt,
        template,
        args: cmdTokens.slice(1)
      };
    }

    // detach-client
    if (name === 'detach-client') {
      return {
        kind: 'native',
        name: 'detach-client',
        args: cmdTokens.slice(1)
      };
    }

    // Unsupported UI commands
    if (UNSUPPORTED_COMMANDS.has(name)) {
      return {
        kind: 'native',
        name: 'unsupported',
        command: name,
        args: cmdTokens.slice(1)
      };
    }

    // Check nested commands inside { ... } (e.g. if-shell -F 1 { copy-mode })
    for (const t of cmdTokens.slice(1)) {
      if (typeof t === 'string' && t.startsWith('{') && t.endsWith('}')) {
        const inner = t.slice(1, -1).trim();
        const innerClassification = classifyCommand(inner);
        if (innerClassification.kind === 'native') {
          return innerClassification;
        }
      }
    }
  }

  return { kind: 'run' };
}

/**
 * Pure key router state machine (§2.2)
 */
export function createKeyRouter({ tables = {}, prefix = 'C-b', prefix2 = null, repeatTime = 500, now } = {}) {
  let table = 'root';
  let inRepeat = false;
  let repeatUntil = 0;

  const getTime = typeof now === 'function' ? now : () => Date.now();
  const repTime = Number(repeatTime) || 500;
  const p1 = prefix || 'C-b';
  const p2 = prefix2 && prefix2 !== 'None' ? prefix2 : null;

  function reset() {
    table = 'root';
    inRepeat = false;
    repeatUntil = 0;
  }

  function state() {
    const isRepeat = inRepeat && getTime() <= repeatUntil;
    return {
      table,
      inRepeat: isRepeat
    };
  }

  function executeBinding(binding) {
    const currentTime = getTime();
    if (binding.repeat) {
      inRepeat = true;
      repeatUntil = currentTime + repTime;
    } else {
      reset();
    }

    const cmd = binding.command.trim();

    // send-prefix / send-prefix -2
    if (cmd === 'send-prefix' || cmd === 'send-prefix -2') {
      return { consume: true, run: cmd, sendPrefix: true };
    }

    // switch-client -T <name>
    const switchMatch = cmd.match(/^switch-client\s+(?:.*?\s+)?-T\s+(\S+)$/);
    if (switchMatch) {
      table = switchMatch[1];
      inRepeat = false;
      repeatUntil = 0;
      return { consume: true };
    }

    const c = classifyCommand(cmd);
    if (c.kind === 'native') {
      return { consume: true, native: c };
    }
    return { consume: true, run: cmd };
  }

  function handle(keyName) {
    const currentTime = getTime();

    // If we're out of root in repeat mode and repeat window expired, reset to root
    if (table !== 'root' && inRepeat && currentTime > repeatUntil) {
      reset();
    }

    // Step 1: Root table
    if (table === 'root') {
      if (keyName === p1 || (p2 && keyName === p2)) {
        table = 'prefix';
        inRepeat = false;
        repeatUntil = 0;
        return { consume: true };
      }

      if (tables.root && tables.root.has(keyName)) {
        return executeBinding(tables.root.get(keyName));
      }

      return { consume: false };
    }

    // Step 2 & 3: Table lookup (prefix or custom table)
    const currentTableMap = tables[table];
    const binding = currentTableMap ? currentTableMap.get(keyName) : null;

    if (binding) {
      return executeBinding(binding);
    }

    // Step 3 / 4: Not found
    if (inRepeat) {
      // In repeat window, unbound key resets to root and re-runs Step 1 so it reaches the pane
      reset();
      return handle(keyName);
    }

    // Freshly after prefix: swallow unbound key (including Escape)
    reset();
    return { consume: true };
  }

  return {
    handle,
    reset,
    state
  };
}
