import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const rootDir = path.resolve(__dirname, '..');
const frontendVendorDir = path.resolve(__dirname, '../../../frontend/vendor');

const pkgJson = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf8'));

// Delete deprecated/removed canvas addon if present
const canvasPath = path.join(frontendVendorDir, 'xterm-addon-canvas.js');
if (fs.existsSync(canvasPath)) {
  fs.unlinkSync(canvasPath);
  console.log('Removed obsolete xterm-addon-canvas.js');
}

const copyFiles = [
  {
    src: path.join(rootDir, 'node_modules/@xterm/xterm/lib/xterm.js'),
    dest: path.join(frontendVendorDir, 'xterm.js')
  },
  {
    src: path.join(rootDir, 'node_modules/@xterm/xterm/css/xterm.css'),
    dest: path.join(frontendVendorDir, 'xterm.css')
  },
  {
    src: path.join(rootDir, 'node_modules/@xterm/addon-fit/lib/addon-fit.js'),
    dest: path.join(frontendVendorDir, 'xterm-addon-fit.js')
  },
  {
    src: path.join(rootDir, 'node_modules/@xterm/addon-webgl/lib/addon-webgl.js'),
    dest: path.join(frontendVendorDir, 'xterm-addon-webgl.js')
  },
  {
    src: path.join(rootDir, 'node_modules/@xterm/addon-unicode11/lib/addon-unicode11.js'),
    dest: path.join(frontendVendorDir, 'xterm-addon-unicode11.js')
  }
];

for (const { src, dest } of copyFiles) {
  if (!fs.existsSync(src)) {
    throw new Error(`Source file not found: ${src}. Run npm install in tests/terminal first.`);
  }
  fs.copyFileSync(src, dest);
  console.log(`Copied ${path.basename(src)} -> ${path.relative(rootDir, dest)}`);
}

const versions = {};
const pkgs = [
  '@xterm/xterm',
  '@xterm/headless',
  '@xterm/addon-fit',
  '@xterm/addon-webgl',
  '@xterm/addon-unicode11'
];

for (const pkg of pkgs) {
  try {
    const pJson = JSON.parse(
      fs.readFileSync(path.join(rootDir, 'node_modules', pkg, 'package.json'), 'utf8')
    );
    versions[pkg] = pJson.version;
  } catch (e) {
    versions[pkg] = pkgJson.dependencies[pkg] || 'unknown';
  }
}

const versionsPath = path.join(frontendVendorDir, 'VERSIONS.json');
fs.writeFileSync(versionsPath, JSON.stringify(versions, null, 2) + '\n', 'utf8');
console.log(`Wrote ${versionsPath}`);
