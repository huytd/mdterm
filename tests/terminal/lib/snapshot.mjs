import headlessPkg from '@xterm/headless';
import unicodePkg from '@xterm/addon-unicode11';
import { buildTerminalOptions } from '../../../frontend/js/terminal-core.js';

const Terminal = headlessPkg.Terminal || headlessPkg;
const Unicode11Addon = unicodePkg.Unicode11Addon || unicodePkg;

/**
 * Creates and configures a headless terminal instance matching mdterm settings.
 */
export function createHeadlessTerminal(cols = 80, rows = 24, cfg = {}) {
  const options = buildTerminalOptions(cfg);
  const term = new Terminal({
    ...options,
    cols,
    rows,
    allowProposedApi: true,
  });

  const unicode11 = new Unicode11Addon();
  term.loadAddon(unicode11);
  if (term.unicode) {
    term.unicode.activeVersion = '11';
  }

  return term;
}

/**
 * Returns a complete snapshot of the terminal visible screen.
 * Format: {cols, rows, cursor, alt, lines:[text], cells:[[{ch,w,fg,bg,fgMode,bgMode,bold,italic,underline,inverse,strike}]]}
 */
export function snapshot(term) {
  const buf = term.buffer.active;
  const cols = term.cols;
  const rows = term.rows;
  const cursor = { x: buf.cursorX, y: buf.cursorY };
  const alt = buf.type === 'alternate';
  const lines = [];
  const cells = [];

  for (let y = 0; y < rows; y++) {
    const line = buf.getLine(y);
    lines.push(line ? line.translateToString(true) : '');

    const rowCells = [];
    for (let x = 0; x < cols; x++) {
      const cell = line ? line.getCell(x) : null;
      if (!cell) {
        rowCells.push({
          ch: ' ',
          w: 1,
          fg: -1,
          bg: -1,
          fgMode: 0,
          bgMode: 0,
          bold: false,
          italic: false,
          underline: false,
          inverse: false,
          strike: false,
        });
      } else {
        rowCells.push({
          ch: cell.getChars(),
          w: cell.getWidth(),
          fg: cell.getFgColor(),
          bg: cell.getBgColor(),
          fgMode: cell.getFgColorMode(),
          bgMode: cell.getBgColorMode(),
          bold: Boolean(cell.isBold()),
          italic: Boolean(cell.isItalic()),
          underline: Boolean(cell.isUnderline()),
          inverse: Boolean(cell.isInverse()),
          strike: Boolean(cell.isStrikethrough ? cell.isStrikethrough() : false),
        });
      }
    }
    cells.push(rowCells);
  }

  return {
    cols,
    rows,
    cursor,
    alt,
    lines,
    cells,
  };
}
