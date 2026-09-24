// Test-only stand-in for src-tauri/src/tmux: spawns `tmux -C` against an
// isolated socket and produces the same channel frames the Rust backend does
// ([1][pane u32 LE][bytes] for output, [2][json] for everything else), so the
// real frontend client can be driven end-to-end in a browser.
import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';

let socket = 'mdterm-e2e';
let conf = '/dev/null';

/** Each test gets its own server so a dying one can't race the next test. */
export function useSocket(name, confPath) {
  socket = name;
  conf = confPath || '/dev/null';
}

export function tmux(...args) {
  return execFileSync('tmux', ['-L', socket, '-f', conf, ...args], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
}

/** Kills the current test server and removes its socket file. */
export function killServer() {
  let path = '';
  try {
    path = tmux('display', '-p', '#{socket_path}').trim();
  } catch {}
  try { tmux('kill-server'); } catch {}
  if (path) fs.rmSync(path, { force: true });
}

export function tmuxAvailable() {
  try {
    execFileSync('tmux', ['-V']);
    return true;
  } catch {
    return false;
  }
}

function decodeOctal(buf) {
  const out = [];
  for (let i = 0; i < buf.length; i++) {
    const b = buf[i];
    if (b === 0x5c && i + 3 < buf.length &&
        [1, 2, 3].every(k => buf[i + k] >= 0x30 && buf[i + k] <= 0x37)) {
      out.push((buf[i + 1] - 48) * 64 + (buf[i + 2] - 48) * 8 + (buf[i + 3] - 48));
      i += 3;
    } else {
      out.push(b);
    }
  }
  return Buffer.from(out);
}

const id = (s) => Number(String(s).replace(/^[%@$]/, ''));

export class ControlConnection {
  constructor(session, deliver) {
    this.deliver = deliver;
    this.pending = [];
    this.block = null;
    this.buf = Buffer.alloc(0);
    this.hasExited = false;
    this.proc = spawn('tmux', ['-L', socket, '-f', conf, '-C', 'attach-session', '-t', session], {
      stdio: ['pipe', 'pipe', 'ignore'],
    });
    this.proc.stdin?.on('error', () => {});
    this.proc.stdout.on('data', (d) => this.feed(d));
    this.exited = new Promise((resolve) => this.proc.on('exit', () => {
      this.hasExited = true;
      resolve();
    }));
  }

  write(commands, tags) {
    if (this.hasExited || !this.proc.stdin || this.proc.stdin.destroyed || this.proc.stdin.writableEnded) return;
    commands.forEach((_, i) => this.pending.push(tags[i] || 0));
    try {
      this.proc.stdin.write(commands.join(' ; ') + '\n');
    } catch (_) {}
  }

  sendKeys(pane, bytes) {
    if (this.hasExited || !this.proc.stdin || this.proc.stdin.destroyed || this.proc.stdin.writableEnded) return;
    for (let i = 0; i < bytes.length; i += 256) {
      const hex = Array.from(bytes.slice(i, i + 256), b => b.toString(16).padStart(2, '0')).join(' ');
      this.write([`send-keys -H -t %${pane} ${hex}`], [0]);
    }
  }

  json(v) {
    this.deliver([2, ...Buffer.from(JSON.stringify(v))]);
  }

  feed(chunk) {
    this.buf = Buffer.concat([this.buf, chunk]);
    let nl;
    while ((nl = this.buf.indexOf(0x0a)) >= 0) {
      let line = this.buf.subarray(0, nl);
      this.buf = this.buf.subarray(nl + 1);
      if (line[line.length - 1] === 0x0d) line = line.subarray(0, line.length - 1);
      this.line(line);
    }
  }

  line(raw) {
    const text = raw.toString('utf8');
    if (this.block) {
      const m = /^%(end|error) \S+ (\d+)/.exec(text);
      if (m && Number(m[2]) === this.block.number) {
        const b = this.block;
        this.block = null;
        if (b.fromClient) {
          const tag = this.pending.shift() || 0;
          if (tag) this.json({ t: 'reply', tag, ok: m[1] === 'end', lines: b.lines });
        }
        return;
      }
      this.block.lines.push(text);
      return;
    }
    const extended = text.startsWith('%extended-output ');
    if (text.startsWith('%output ') || extended) {
      const rest = raw.subarray(extended ? 17 : 8);
      const sp = rest.indexOf(0x20);
      const pane = id(rest.subarray(0, sp).toString());
      const start = extended ? rest.indexOf(' : ') + 3 : sp + 1;
      const data = decodeOctal(rest.subarray(start));
      const head = Buffer.alloc(5);
      head[0] = 1;
      head.writeUInt32LE(pane, 1);
      this.deliver([...head, ...data]);
      return;
    }
    const [name, ...args] = text.split(' ');
    switch (name) {
      case '%begin':
        this.block = { number: Number(args[1]), fromClient: (Number(args[2]) & 1) === 1, lines: [] };
        break;
      case '%layout-change':
        this.json({ t: 'layout', window: id(args[0]), layout: args[1], visible: args[2], flags: args[3] || '' });
        break;
      case '%window-add':
        this.json({ t: 'window-add', window: id(args[0]) });
        break;
      case '%window-close':
      case '%unlinked-window-close':
        this.json({ t: 'window-close', window: id(args[0]) });
        break;
      case '%window-renamed':
        this.json({ t: 'window-renamed', window: id(args[0]), name: args.slice(1).join(' ') });
        break;
      case '%window-pane-changed':
        this.json({ t: 'window-pane-changed', window: id(args[0]), pane: id(args[1]) });
        break;
      case '%session-window-changed':
        this.json({ t: 'session-window-changed', session: id(args[0]), window: id(args[1]) });
        break;
      case '%session-changed':
        this.json({ t: 'session-changed', session: id(args[0]), name: args.slice(1).join(' ') });
        break;
      case '%pause':
        this.write([`refresh-client -A '%${id(args[0])}:continue'`], [0]);
        break;
      case '%pane-mode-changed':
        this.json({ t: 'pane-mode-changed', pane: id(args[0]) });
        break;
      case '%exit':
        this.json({ t: 'exit', reason: args.join(' ') || null });
        break;
      default:
        break;
    }
  }

  kill() {
    try { this.proc.kill(); } catch {}
  }
}
