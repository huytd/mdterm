#!/usr/bin/env node

/**
 * import-recording.mjs
 * Converts MDTERM_RECORD_DIR recordings (<id>-<timestamp>.bin + .events.jsonl)
 * into a test fixture in tests/terminal/fixtures/<fixture-name>/.
 *
 * Usage:
 *   node tests/terminal/scripts/import-recording.mjs <path-to-bin-or-prefix> <fixture-name> [--cols 80] [--rows 24]
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHeadlessTerminal, snapshot } from '../lib/snapshot.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const fixturesDir = path.resolve(__dirname, '../fixtures');
const vendorVersionsPath = path.resolve(__dirname, '../../../frontend/vendor/VERSIONS.json');

const args = process.argv.slice(2);
if (args.length < 2) {
  console.error('Usage: node scripts/import-recording.mjs <bin-or-prefix> <fixture-name> [--cols 80] [--rows 24]');
  process.exit(1);
}

let inputTarget = args[0];
let fixtureName = args[1];
let initialCols = 80;
let initialRows = 24;

for (let i = 2; i < args.length; i++) {
  if (args[i] === '--cols' && args[i + 1]) {
    initialCols = parseInt(args[++i], 10);
  } else if (args[i] === '--rows' && args[i + 1]) {
    initialRows = parseInt(args[++i], 10);
  }
}

// Normalize bin and events path
let binPath = inputTarget;
let eventsPath = inputTarget;

if (binPath.endsWith('.bin')) {
  eventsPath = binPath.slice(0, -4) + '.events.jsonl';
} else if (binPath.endsWith('.events.jsonl')) {
  eventsPath = binPath;
  binPath = binPath.slice(0, -13) + '.bin';
} else {
  binPath = inputTarget + '.bin';
  eventsPath = inputTarget + '.events.jsonl';
}

if (!fs.existsSync(binPath)) {
  console.error(`Error: Binary recording file not found: ${binPath}`);
  process.exit(1);
}

const targetDir = path.join(fixturesDir, fixtureName);
fs.mkdirSync(targetDir, { recursive: true });

// Copy output.bin
const outputBinPath = path.join(targetDir, 'output.bin');
fs.copyFileSync(binPath, outputBinPath);
console.log(`Copied recording to ${outputBinPath}`);

// Parse events.jsonl
const events = [];
if (fs.existsSync(eventsPath)) {
  const lines = fs.readFileSync(eventsPath, 'utf8').split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const parsed = JSON.parse(trimmed);
      if (parsed.type === 'resize') {
        events.push({
          type: 'resize',
          offset: Number(parsed.offset) || 0,
          cols: Number(parsed.cols) || initialCols,
          rows: Number(parsed.rows) || initialRows,
        });
      }
    } catch (e) {
      console.warn(`Warning: failed to parse event line: ${trimmed}`);
    }
  }
}

// Read xterm version
let xtermVersion = undefined;
if (fs.existsSync(vendorVersionsPath)) {
  try {
    const vJson = JSON.parse(fs.readFileSync(vendorVersionsPath, 'utf8'));
    xtermVersion = vJson['@xterm/xterm'] || vJson['@xterm/headless'];
  } catch (e) {}
}

// Write meta.json
const meta = {
  cols: initialCols,
  rows: initialRows,
  command: `imported from ${path.basename(binPath)}`,
  tmux: false,
  xtermVersion,
  events,
};

const metaPath = path.join(targetDir, 'meta.json');
fs.writeFileSync(metaPath, JSON.stringify(meta, null, 2) + '\n', 'utf8');
console.log(`Wrote metadata to ${metaPath}`);

// Generate expected.json golden
const binData = fs.readFileSync(outputBinPath);
const term = createHeadlessTerminal(initialCols, initialRows);
const sortedEvents = [...events].sort((a, b) => a.offset - b.offset);

let eventIdx = 0;
let offset = 0;
const decoder = new TextDecoder('utf-8');

while (offset < binData.length) {
  let nextEventOffset = Infinity;
  if (eventIdx < sortedEvents.length) {
    nextEventOffset = sortedEvents[eventIdx].offset;
  }

  if (offset === nextEventOffset) {
    const ev = sortedEvents[eventIdx];
    if (ev.type === 'resize') {
      term.resize(ev.cols, ev.rows);
    }
    eventIdx++;
    continue;
  }

  const chunkSize = offset < nextEventOffset ? Math.min(binData.length - offset, nextEventOffset - offset) : binData.length - offset;
  const slice = binData.subarray(offset, offset + chunkSize);
  const isEnd = (offset + chunkSize >= binData.length);
  const text = decoder.decode(slice, { stream: !isEnd });
  await new Promise(r => term.write(text, r));
  offset += chunkSize;
}

while (eventIdx < sortedEvents.length && sortedEvents[eventIdx].offset <= offset) {
  const ev = sortedEvents[eventIdx];
  if (ev.type === 'resize') {
    term.resize(ev.cols, ev.rows);
  }
  eventIdx++;
}

const snap = snapshot(term);
const expectedPath = path.join(targetDir, 'expected.json');
fs.writeFileSync(expectedPath, JSON.stringify(snap, null, 2) + '\n', 'utf8');
console.log(`Generated golden expected.json at ${expectedPath}`);
console.log(`Successfully created fixture '${fixtureName}'! Run 'npm test' to verify.`);
