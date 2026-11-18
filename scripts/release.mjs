import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { copyFile, mkdir, mkdtemp, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const repo = 'MaKammi/MeetingPal';
const version = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8')).version;
const config = JSON.parse(await readFile(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const tag = `v${version}`;
const revision = process.env.GITHUB_SHA;
const inputs = path.join(root, 'runtime-data/release-input');
const assets = path.join(root, 'runtime-data/release-output', tag);
const releaseNotes = path.join(root, 'docs/releases', `${version}.md`);
const rust = await readFile(path.join(root, 'src-tauri/Cargo.toml'), 'utf8');
if (!/^\d+\.\d+\.\d+$/.test(version) || version !== config.version || rust.match(/^version = "([^"]+)"/m)?.[1] !== version || process.env.GITHUB_REF_NAME !== tag || !/^[a-f0-9]{40}$/.test(revision ?? '')) {
  throw new Error('A stable version tag, matching application versions and a commit revision are required.');
}
await stat(releaseNotes);

// Each artifact must contain exactly the files for its platform. macOS updater
// archives have identical input names on the two runners and distinct release names.
const builds = [
  { name: 'windows-x64', platform: 'win32', arch: 'x64', files: [
    ['.exe', 'x64-setup.exe', ['windows-x86_64']],
  ] },
  { name: 'macos-arm64', platform: 'darwin', arch: 'arm64', files: [
    ['.dmg', 'aarch64.dmg'], ['.app.tar.gz', 'aarch64.app.tar.gz', ['darwin-aarch64']],
  ] },
  { name: 'macos-x64', platform: 'darwin', arch: 'x64', files: [
    ['.dmg', 'x64.dmg'], ['.app.tar.gz', 'x64.app.tar.gz', ['darwin-x86_64']],
  ] },
  { name: 'linux-x64', platform: 'linux', arch: 'x64', files: [
    ['.deb', 'amd64.deb', ['linux-x86_64-deb']],
    ['.AppImage', 'amd64.AppImage', ['linux-x86_64-appimage', 'linux-x86_64']],
    ['appimage-notices.tar.gz', 'AppImage-notices.tar.gz'],
  ] },
];

async function sha256(file) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}

async function prepare() {
  if (!process.env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('Missing release key');
  await mkdir(assets, { recursive: true });
  const temp = await mkdtemp(path.join(tmpdir(), 'meetingpal-verify-'));
  const publicKey = path.join(temp, 'public.key');
  await writeFile(publicKey, Buffer.from(config.plugins.updater.pubkey, 'base64'));
  const platforms = {};
  for (const build of builds) {
    const dir = path.join(inputs, `meetingpal-${build.name}`);
    const manifest = JSON.parse(await readFile(path.join(dir, 'build-info.json'), 'utf8'));
    if (manifest.version !== version || manifest.revision !== revision || manifest.platform !== build.platform || manifest.architecture !== build.arch || manifest.files.length !== build.files.length) {
      throw new Error(`Unexpected build manifest for ${build.name}`);
    }
    for (const [suffix, releaseSuffix, targets] of build.files) {
      const matches = manifest.files.filter(f => f.path.endsWith(suffix));
      if (matches.length !== 1) throw new Error(`Expected one ${suffix} for ${build.name}`);
      const [file] = matches;
      const source = path.resolve(dir, file.path);
      if (!source.startsWith(dir + path.sep) || file.bytes !== (await stat(source)).size || file.sha256 !== await sha256(source)) {
        throw new Error(`Package integrity check failed for ${build.name} ${suffix}`);
      }
      const name = `MeetingPal_${version}_${releaseSuffix}`;
      const destination = path.join(assets, name);
      await copyFile(source, destination);
      if (!targets) continue;
      execFileSync(process.execPath, [path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'signer', 'sign', '--app-version', version, destination]);
      const signature = (await readFile(destination + '.sig', 'utf8')).trim();
      const minisig = Buffer.from(signature, 'base64').toString('utf8');
      const verifyFile = path.join(temp, name + '.minisig');
      await writeFile(verifyFile, minisig);
      execFileSync('minisign', ['-V', '-m', destination, '-p', publicKey, '-x', verifyFile]);
      const trusted = minisig.split(/\r?\n/).find(line => line.startsWith('trusted comment: '));
      if (!trusted?.split('\t').includes(`version:${version}`)) throw new Error(`Unsigned version for ${name}`);
      for (const target of targets) platforms[target] = { url: `https://github.com/${repo}/releases/download/${tag}/${name}`, signature };
    }
  }
  const notes = (await readFile(releaseNotes, 'utf8')).trim();
  await writeFile(path.join(assets, 'latest.json'), JSON.stringify({ version, pub_date: new Date().toISOString(), notes, platforms }, null, 2) + '\n');
  const sums = [];
  for (const name of (await readdir(assets)).filter(n => n !== 'SHA256SUMS').sort()) sums.push(`${await sha256(path.join(assets, name))}  ${name}`);
  await writeFile(path.join(assets, 'SHA256SUMS'), sums.join('\n') + '\n');
  console.log(`Prepared ${tag} for ${Object.keys(platforms).join(', ')}.`);
}

function gh(...args) {
  return execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 }).trim();
}

async function publish() {
  const releases = JSON.parse(gh('api', `repos/${repo}/releases?per_page=100`));
  const existing = releases.find(release => release.tag_name === tag);
  if (existing && !existing.draft) throw new Error(`${tag} is already public`);
  // Keep older tags from replacing the feed for a newer stable version.
  if (releases.some(r => !r.draft && !r.prerelease && /^v\d+\.\d+\.\d+$/.test(r.tag_name) && r.tag_name.localeCompare(tag, 'en', { numeric: true }) > 0)) throw new Error('A newer release is already published');
  if (!existing) gh('release', 'create', tag, '--repo', repo, '--verify-tag', '--draft', '--title', `MeetingPal ${version}`, '--notes-file', releaseNotes);
  const names = (await readdir(assets)).sort();
  gh('release', 'upload', tag, '--repo', repo, '--clobber', ...names.map(n => path.join(assets, n)));
  const draft = JSON.parse(gh('api', `repos/${repo}/releases?per_page=100`)).find(r => r.tag_name === tag && r.draft);
  if (!draft) throw new Error('Release draft missing');
  const uploaded = JSON.parse(gh('api', `repos/${repo}/releases/${draft.id}/assets?per_page=100`));
  if (uploaded.length !== names.length) throw new Error('Incomplete draft release');
  for (const name of names) {
    const asset = uploaded.find(a => a.name === name);
    if (asset?.digest !== `sha256:${await sha256(path.join(assets, name))}`) throw new Error(`GitHub digest mismatch for ${name}`);
  }
  console.log(gh('release', 'edit', tag, '--repo', repo, '--draft=false', '--latest'));
}

switch (process.argv[2]) {
  case 'check-tag': break;
  case 'prepare': await prepare(); break;
  case 'publish': await publish(); break;
  default: throw new Error('Expected check-tag, prepare or publish');
}
