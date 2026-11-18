import { mkdir, readFile, writeFile, copyFile, readdir, access, chmod } from 'node:fs/promises';
import { createReadStream, createWriteStream, existsSync } from 'node:fs';
import { pipeline } from 'node:stream/promises';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';

const root=path.resolve(import.meta.dirname,'..');
const cache=path.join(root,'vendor'); const output=path.join(root,'src-tauri','binaries');
const lock=JSON.parse(await readFile(path.join(root,'resources/runtimes.json'),'utf8'));
const requested=process.argv.find(s=>s.startsWith('--only='))?.slice(7);
const selected=name=>!requested||requested.split(',').includes(name);
await mkdir(cache,{recursive:true}); await mkdir(output,{recursive:true});
const jobs=String(Math.min(4,os.availableParallelism()));
const cmakeWindows='C:/Program Files (x86)/Microsoft Visual Studio/18/BuildTools/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe';
const cmake=process.platform==='win32'&&existsSync(cmakeWindows)?cmakeWindows:'cmake';
async function run(command,args,cwd=root,env=process.env) {
  console.log(path.basename(command),args.join(' '));
  await new Promise((resolve,reject)=>{ const child=spawn(command,args,{cwd,env,stdio:'inherit',windowsHide:true}); child.on('error',reject); child.on('exit',code=>code===0?resolve():reject(new Error(`${command} exited ${code}`))); });
}
async function clone(name,repo,ref) {
  const dir=path.join(cache,name);
  if(!existsSync(path.join(dir,'.git'))) await run('git',['clone','--depth','1','--branch',ref,repo,dir]);
  return dir;
}
async function files(dir) { const out=[]; for(const e of await readdir(dir,{withFileTypes:true})) { const p=path.join(dir,e.name); if(e.isDirectory()) out.push(...await files(p)); else out.push(p); } return out; }
for (const [name,target] of [['whisper','whisper-cli'],['llama','llama-server']]) {
  if(!selected(name)) continue;
  const config=lock[name]; const dir=await clone(name,config.repository,config.tag);
  await run('git',['diff','--exit-code'],dir);
  await run('git',['checkout','--detach',config.commit],dir);
  const buildDir=path.join(dir,name==='llama'?'build-meetingpal':'build');
  await run(cmake,['-S',dir,'-B',buildDir,'-DCMAKE_BUILD_TYPE=Release','-DBUILD_SHARED_LIBS=OFF','-DGGML_OPENMP=OFF','-DGGML_NATIVE=OFF','-DGGML_CUDA=OFF','-DGGML_VULKAN=OFF','-DWHISPER_BUILD_TESTS=OFF','-DLLAMA_BUILD_TESTS=OFF','-DLLAMA_BUILD_UI=OFF','-DLLAMA_USE_PREBUILT_UI=OFF','-DLLAMA_OPENSSL=OFF','-DLLAMA_SUBPROCESS=OFF']);
  await run(cmake,['--build',buildDir,'--config','Release','--target',target,'--parallel',jobs]);
  const binary=(await files(path.join(buildDir,'bin'))).find(p=>path.basename(p)===target+(process.platform==='win32'?'.exe':''));
  if(!binary) throw new Error(`Missing ${target}`);
  await copyFile(binary,path.join(output,path.basename(binary)));
  await copyFile(path.join(dir,'LICENSE'),path.join(output,`${name}-LICENSE.txt`));
  await run('git',['archive','--format=tar.gz','--output',path.join(output,`${name}-source.tar.gz`),config.commit],dir);
}
if(selected('ffmpeg')) {
  const archive=lock.ffmpeg[process.platform];
  if(process.platform!=='win32') {
    await run(process.execPath,[path.join(root,'scripts/build-codecs.mjs')]);
  } else if(archive) {
    const archivePath=path.join(cache,path.basename(new URL(archive.url).pathname));
    if(!existsSync(archivePath)) { const response=await fetch(archive.url); if(!response.ok) throw new Error(`FFmpeg download: ${response.status}`); await pipeline(response.body,createWriteStream(archivePath)); }
    const hash=createHash('sha256'); for await(const chunk of createReadStream(archivePath))hash.update(chunk);
    if(hash.digest('hex')!==archive.sha256)throw new Error('FFmpeg archive checksum mismatch');
    const unpack=path.join(cache,'ffmpeg-package'); await mkdir(unpack,{recursive:true});
    await run('tar',['-xf',archivePath,'-C',unpack]);
    const all=await files(unpack);
    for(const name of ['ffmpeg','ffprobe']) { const binary=all.find(p=>path.basename(p)===name+(process.platform==='win32'?'.exe':'')); if(!binary)throw new Error(`Missing ${name}`); await copyFile(binary,path.join(output,path.basename(binary))); }
    for(const file of all.filter(p=>/license|copying/i.test(path.basename(p))))await copyFile(file,path.join(output,'ffmpeg-'+path.basename(file)));
    await writeFile(path.join(output,'ffmpeg-provenance.json'),JSON.stringify(archive,null,2));
  } else throw new Error('Unsupported platform for runtime build');
}
await copyFile(path.join(root,'resources/runtimes.json'),path.join(output,'runtime-versions.json'));
if(process.platform!=='win32')for(const name of ['ffmpeg','ffprobe','whisper-cli','llama-server'])if(existsSync(path.join(output,name)))await chmod(path.join(output,name),0o755);
if(selected('ffmpeg'))await run(path.join(output,'ffmpeg'+(process.platform==='win32'?'.exe':'')),['-hide_banner','-version']);
console.log('Bundled runtimes ready:',output);
