import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHeadlessTerminal, snapshot } from './lib/snapshot.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const fixturesDir = path.join(__dirname, 'fixtures');

function mulberry32(seed) {
  return function () {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

async function replayWithSeed(data, meta, seed) {
  const term = createHeadlessTerminal(meta.cols, meta.rows);
  const decoder = new TextDecoder('utf-8');
  const prng = mulberry32(seed);
  const events = [...(meta.events || [])].sort((a, b) => a.offset - b.offset);
  let eventIdx = 0;
  let offset = 0;

  while (offset < data.length) {
    let nextEventOffset = Infinity;
    if (eventIdx < events.length) {
      nextEventOffset = events[eventIdx].offset;
    }

    if (offset === nextEventOffset) {
      const ev = events[eventIdx];
      if (ev.type === 'resize') {
        term.resize(ev.cols, ev.rows);
      }
      eventIdx++;
      continue;
    }

    const maxChunk = Math.floor(prng() * 255) + 1;
    const available = Math.min(data.length - offset, maxChunk);
    const chunkSize =
      offset < nextEventOffset
        ? Math.min(available, nextEventOffset - offset)
        : available;

    const slice = data.subarray(offset, offset + chunkSize);
    const isEnd = offset + chunkSize >= data.length;
    const text = decoder.decode(slice, { stream: !isEnd });
    await new Promise((resolve) => term.write(text, resolve));
    offset += chunkSize;
  }

  while (eventIdx < events.length && events[eventIdx].offset <= offset) {
    const ev = events[eventIdx];
    if (ev.type === 'resize') {
      term.resize(ev.cols, ev.rows);
    }
    eventIdx++;
  }

  return snapshot(term);
}

const fixtureNames = fs
  .readdirSync(fixturesDir, { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name)
  .sort();

for (const name of fixtureNames) {
  test(`fixture: ${name}`, async () => {
    const fixtureDir = path.join(fixturesDir, name);
    const metaPath = path.join(fixtureDir, 'meta.json');
    const binPath = path.join(fixtureDir, 'output.bin');

    assert.ok(fs.existsSync(metaPath), `Missing meta.json in ${fixtureDir}`);
    assert.ok(fs.existsSync(binPath), `Missing output.bin in ${fixtureDir}`);

    const meta = JSON.parse(fs.readFileSync(metaPath, 'utf8'));
    const data = fs.readFileSync(binPath);

    // 1. Replay under 20 deterministic seeds and assert chunk invariance
    const snapshots = [];
    for (let seed = 1; seed <= 20; seed++) {
      const snap = await replayWithSeed(data, meta, seed);
      snapshots.push(snap);
    }

    const baseSnapshot = snapshots[0];
    for (let s = 1; s < snapshots.length; s++) {
      assert.deepStrictEqual(
        snapshots[s],
        baseSnapshot,
        `Chunking bug detected: seed ${s + 1} produced different snapshot than seed 1 for fixture ${name}`
      );
    }

    // 2. Assert no fixture screen contains U+FFFD
    for (let y = 0; y < baseSnapshot.lines.length; y++) {
      const line = baseSnapshot.lines[y];
      assert.ok(
        !line.includes('\uFFFD'),
        `Fixture ${name} contains U+FFFD at line ${y}: ${JSON.stringify(line)}`
      );
    }

    // 3. Golden comparison / update
    const expectedPath = path.join(fixtureDir, 'expected.json');
    const isUpdate =
      process.argv.includes('--update') || process.env.UPDATE === '1';

    if (isUpdate || !fs.existsSync(expectedPath)) {
      fs.writeFileSync(
        expectedPath,
        JSON.stringify(baseSnapshot, null, 2) + '\n'
      );
    } else {
      const expected = JSON.parse(fs.readFileSync(expectedPath, 'utf8'));
      assert.deepStrictEqual(
        baseSnapshot,
        expected,
        `Golden snapshot mismatch against expected.json in ${name}`
      );
    }

    // 4. tmux-oracle comparison if present
    const oraclePath = path.join(fixtureDir, 'tmux-oracle.json');
    if (fs.existsSync(oraclePath)) {
      const oracle = JSON.parse(fs.readFileSync(oraclePath, 'utf8'));

      if (oracle.panes && Array.isArray(oracle.panes)) {
        for (const pane of oracle.panes) {
          const oracleLines = pane.text.split('\n');
          for (let r = 0; r < pane.height; r++) {
            const screenY = pane.top + r;
            let actualSlice = '';
            for (let c = pane.left; c < pane.left + pane.width; c++) {
              actualSlice += baseSnapshot.cells[screenY]?.[c]?.ch || ' ';
            }
            const actualTrimmed = actualSlice.trimEnd();
            const expectedTrimmed = (oracleLines[r] || '').trimEnd();

            if (actualTrimmed !== expectedTrimmed) {
              console.error(
                `\n[Diff] Pane ${pane.id} Row ${r} (screen line ${screenY}) in fixture ${name}:`
              );
              console.error(`  tmux oracle:     "${expectedTrimmed}"`);
              console.error(`  headless replay: "${actualTrimmed}"`);
              assert.strictEqual(
                actualTrimmed,
                expectedTrimmed,
                `Pane ${pane.id} row ${r} does not match tmux oracle in ${name}`
              );
            }
          }
        }
      }

      if (oracle.status) {
        const statusY = baseSnapshot.rows - 1;
        const actualStatus = (baseSnapshot.lines[statusY] || '').trimEnd();
        const expectedStatus = oracle.status.trimEnd();

        if (
          !actualStatus.startsWith(expectedStatus) &&
          actualStatus !== expectedStatus
        ) {
          console.error(`\n[Diff] Status line in fixture ${name} (screen line ${statusY}):`);
          console.error(`  tmux oracle:     "${expectedStatus}"`);
          console.error(`  headless replay: "${actualStatus}"`);
          assert.ok(
            actualStatus.startsWith(expectedStatus),
            `Status line in ${name} did not match tmux oracle`
          );
        }
      }
    }
  });
}
