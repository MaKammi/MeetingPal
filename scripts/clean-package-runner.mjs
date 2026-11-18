// Only ephemeral GitHub runners: keep release compiler dependencies and bundled
// runtimes; discard temporary source/build trees before cache compression.
import { rm } from 'node:fs/promises';
import path from 'node:path';

if (process.env.GITHUB_ACTIONS !== 'true') {
  throw new Error('Package cleanup is restricted to GitHub Actions runners.');
}
const root = path.resolve(import.meta.dirname, '..');
if (path.resolve(process.env.GITHUB_WORKSPACE ?? '') !== root) {
  throw new Error('Unexpected GitHub workspace; refusing cleanup.');
}
const targets = {
  'before-build': ['vendor', 'src-tauri/target/debug'],
  'after-upload': ['src-tauri/target/release/bundle'],
}[process.argv[2]];
if (!targets) throw new Error('Expected before-build or after-upload.');
for (const relative of targets) {
  const target = path.resolve(root, relative);
  if (!target.startsWith(root + path.sep)) throw new Error('Cleanup outside workspace.');
  console.log(`Removing disposable build directory: ${target}`);
  await rm(target, { recursive: true, force: true });
}
