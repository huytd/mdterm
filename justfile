# Default recipe
default:
    @just --list

# L1 and L2 terminal tests
test-term:
    cargo test -p mdterm && cd tests/terminal && npm test

# L3 Playwright visual and geometry tests
test-term-visual:
    cd tests/terminal && npm run test:visual

# Record all deterministic scenarios into fixtures
record:
    ./tests/terminal/record-all.sh

# Sync vendored xterm assets from node_modules
vendor-sync:
    cd tests/terminal && npm run vendor:sync
