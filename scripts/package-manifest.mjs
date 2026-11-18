import { readdir, readFile, stat, writeFile } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const output = path.join(root, 'src-tauri/target/release/bundle');
const revision = process.env.GITHUB_SHA;
if (!/^[a-f0-9]{40}$/.test(revision ?? '')) throw new Error('Missing source commit.');
async function installers(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    // .app and AppDir are staging trees, already contained in the installers.
    if (entry.isDirectory() && !/\.(app|AppDir)$/.test(entry.name)) files.push(...await installers(file));
    else if (entry.isFile() && (/\.(exe|dmg|deb|AppImage|app\.tar\.gz)$/.test(entry.name) || entry.name === 'appimage-notices.tar.gz')) files.push(file);
  }
  return files;
}
const files = [];
for (const file of (await installers(output)).sort()) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  files.push({ path: path.relative(output, file).split(path.sep).join('/'), bytes: (await stat(file)).size, sha256: hash.digest('hex') });
}
if (!files.length) throw new Error('No installers were produced.');
const { version } = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
await writeFile(path.join(output, 'build-info.json'), JSON.stringify({ version, revision, platform: process.platform, architecture: process.arch, files }, null, 2) + '\n');
console.log(`Recorded ${files.length} installer(s) from ${revision}.`);
