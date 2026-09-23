# mdterm Terminal Test Harness

Automated testing framework for terminal emulation and rendering accuracy in `mdterm`.

The test harness consists of three layers:
- **L1: Rust Unit & Integration Tests** (`cargo test -p mdterm`)
  Verifies low-level PTY stream reader batching, coalescing, and exact UTF-8 byte preservation without loss or corruption across arbitrary chunk split boundaries.
- **L2: Headless Replay Conformance** (`tests/terminal/replay.test.mjs`, `npm test`)
  Replays recorded PTY byte streams (`output.bin`) into `@xterm/headless` across 20 deterministic pseudo-random chunking seeds, verifying chunk invariance, absence of replacement characters (`U+FFFD`), and exact match against golden buffer state (`expected.json`). For tmux fixtures, verifies each pane rectangle and status line against the tmux oracle (`tmux-oracle.json`).
- **L3: Visual & Geometry Tests** (`tests/terminal/visual/`, `npm run test:visual`)
  Uses `@playwright/test` with WebKit to verify container fit geometry across 20 seeded sizes and line heights (1.0, 1.2, 1.5), debounced resize bursts (30 resizes within 200ms collapsing to 1), and pixel goldens across DOM and WebGL renderers.

---

## Prerequisites

- Node.js (>= 20) and npm
- Rust (stable toolchain)
- tmux (>= 3.2, for re-recording tmux oracle fixtures)
- Playwright WebKit (`npx playwright install webkit`)

---

## Running Tests

### Fast Test (L1 + L2)
Using `just`:
```bash
just test-term
```
Or manually:
```bash
cargo test -p mdterm
cd tests/terminal && npm test
```

### Visual & Geometry Test (L3)
Using `just`:
```bash
just test-term-visual
```
Or manually:
```bash
cd tests/terminal && npm run test:visual
```

### Update Goldens
To update expected buffer goldens in L2:
```bash
cd tests/terminal && npm test -- --update
```
To update screenshot goldens in L3:
```bash
cd tests/terminal && npx playwright test --update-snapshots
```

---

## Adding a Fixture

### 1. Write a Scenario Script
Create a deterministic script in `tests/terminal/scenarios/<name>.sh`.
- Use `printf` exclusively (avoid escape timing variations).
- End with an idle `sleep 2` so that the recorder detects quiet output before exit.

Example:
```bash
#!/bin/bash
printf '\033[H\033[2J'
printf '=== Sample Feature Test ===\n'
printf 'Testing special terminal sequences\n'
sleep 2
```

### 2. Record with `term-rec`
Record the fixture using the workspace tool `term-rec`:

```bash
# Non-tmux scenario (direct PTY at 80x24)
cargo run -p term-rec -- --cols 80 --rows 24 --out tests/terminal/fixtures/<name> -- /bin/bash tests/terminal/scenarios/<name>.sh

# tmux scenario (captures panes and status bar oracle)
cargo run -p term-rec -- --cols 80 --rows 24 --out tests/terminal/fixtures/<name> --tmux -- /bin/bash tests/terminal/scenarios/<name>.sh

# Scenario with mid-recording resize
cargo run -p term-rec -- --cols 80 --rows 24 --out tests/terminal/fixtures/<name> --tmux --resize-after-ms 400 --resize 100x30 -- /bin/bash tests/terminal/scenarios/<name>.sh
```

To re-record all scenarios:
```bash
just record
# or: ./tests/terminal/record-all.sh
```

---

## Importing Real Recordings (`MDTERM_RECORD_DIR`)

When debugging live terminal rendering glitches in `mdterm`, you can record raw PTY sessions by setting `MDTERM_RECORD_DIR`:

```bash
MDTERM_RECORD_DIR=/tmp/mdterm-recs cargo tauri dev
```

During execution, `pty_stream` writes:
- `<session_id>-<unix_ts>.bin`: raw PTY output stream
- `<session_id>-<unix_ts>.events.jsonl`: stream events (e.g. resizes) with byte offset:
  ```json
  {"t":145,"type":"resize","offset":1024,"cols":100,"rows":30}
  ```

### Converting a Recording into a Test Fixture
Use `scripts/import-recording.mjs` to convert the recording files into a fixture directory:

```bash
node tests/terminal/scripts/import-recording.mjs /tmp/mdterm-recs/<id>-<ts>.bin <fixture-name>
```

The script will:
1. Copy `<id>-<ts>.bin` to `fixtures/<fixture-name>/output.bin`.
2. Parse `<id>-<ts>.events.jsonl` into `fixtures/<fixture-name>/meta.json` with accurate byte offsets.
3. Replay the stream into `@xterm/headless` and generate the initial `expected.json` golden.
4. Integrate the new fixture immediately into `npm test`.

---

## Vendor Sync

When upgrading `@xterm` packages in `tests/terminal/package.json`, synchronize the vendor files into `frontend/vendor/` with:
```bash
just vendor-sync
# or: cd tests/terminal && npm run vendor:sync
```
This updates:
- `frontend/vendor/xterm.js`
- `frontend/vendor/xterm.css`
- `frontend/vendor/xterm-addon-fit.js`
- `frontend/vendor/xterm-addon-unicode11.js`
- `frontend/vendor/xterm-addon-webgl.js`
- `frontend/vendor/VERSIONS.json`
