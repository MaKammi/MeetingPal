// Collect the exact installed dependency notices into the distributable resources.
import { execFileSync } from 'node:child_process';
import { readFile, readdir, mkdir, writeFile, copyFile } from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const output = path.join(root, 'src-tauri/binaries');
await mkdir(output, { recursive: true });
const host = execFileSync('rustc', ['-vV'], { encoding: 'utf8', windowsHide: true }).match(/^host: (.+)$/m)?.[1].trim();
if (!host) throw new Error('Rust host target unavailable');
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--manifest-path', path.join(root, 'src-tauri/Cargo.toml'), '--locked', '--format-version', '1', '--filter-platform', host], { encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 }));
const resolved = new Set(metadata.resolve.nodes.map(p => p.id));
const dependencies = metadata.packages.filter(p => resolved.has(p.id) && p.name !== 'meetingpal').map(p => ({ name: `${p.name} ${p.version} (Rust)`, directory: path.dirname(p.manifest_path), license: p.license, repository: p.repository, licenseFile: p.license_file }));
const npmLock = JSON.parse(await readFile(path.join(root, 'package-lock.json'), 'utf8'));
for (const [location, p] of Object.entries(npmLock.packages)) {
  if (!location || p.dev) continue;
  dependencies.push({ name: `${location.replace(/^node_modules\//, '')} ${p.version} (JavaScript)`, directory: path.join(root, location), license: p.license, repository: p.resolved });
}
let notices = '# MeetingPal third-party notices\n\nThis inventory describes bundled application dependencies. Runtime and codec sources, licenses and build configuration are supplied beside this file. Models are downloaded separately under the licenses shown in the model catalog.\n\n';
let copied = 0;
for (const p of dependencies.sort((a, b) => a.name.localeCompare(b.name))) {
  notices += `## ${p.name}\n\nLicense: ${p.license || 'See license text below'}\nSource: ${p.repository || 'https://crates.io'}\n\n`;
  const entries = await readdir(p.directory, { withFileTypes: true });
  const candidates = new Set(entries.filter(e => e.isFile() && /^(licen[sc]e|copying|notice)([.\-_]|$)/i.test(e.name)).map(e => e.name));
  if (p.licenseFile) candidates.add(p.licenseFile);
  for (const name of candidates) {
    const file = path.resolve(p.directory, name);
    // Package manifests can name a relative license file but must stay in their package.
    if (!file.startsWith(`${path.resolve(p.directory)}${path.sep}`)) continue;
    const text = await readFile(file, 'utf8');
    notices += `### ${name}\n\n${text.trim()}\n\n`;
    copied++;
  }
}
await writeFile(path.join(output, 'THIRD_PARTY_NOTICES.txt'), notices);
await copyFile(path.join(root, 'LICENSE'), path.join(output, 'MeetingPal-LICENSE.txt'));
await copyFile(path.join(root, 'resources/NotoSans-LICENSE.txt'), path.join(output, 'NotoSans-LICENSE.txt'));
await copyFile(path.join(root, 'docs/THIRD_PARTY.md'), path.join(output, 'DISTRIBUTION-NOTES.md'));
console.log(`Bundled notices for ${dependencies.length} dependencies (${copied} license/notice files).`);
