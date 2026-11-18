// Minimal external dependency set: LGPL FFmpeg + BSD libvpx/Opus, all from pinned source.
// Windows is cross-compiled on Ubuntu with mingw-w64; macOS/Linux build natively.
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawn } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
const root=path.resolve(import.meta.dirname,'..');
const target=process.argv.find(a=>a.startsWith('--target='))?.split('=')[1]??process.platform;
const cross=target==='win32';
if(process.platform==='win32') throw new Error('Build source codecs on Ubuntu (cross target win32), or use the Windows-codecs CI artifact.');
const cache=path.join(root,'vendor',`codecs-${target}`), output=path.join(root,'src-tauri/binaries'), prefix=path.join(cache,'prefix');
await mkdir(cache,{recursive:true});await mkdir(output,{recursive:true});await mkdir(prefix,{recursive:true});
const lock=JSON.parse(await readFile(path.join(root,'resources/runtimes.json'),'utf8')).codecs;
const jobs=String(Math.min(4,os.availableParallelism()));
const records=[];
async function run(command,args,cwd=root,env=process.env){ records.push({command,args,cwd:path.relative(root,cwd)}); try { await new Promise((resolve,reject)=>{const c=spawn(command,args,{cwd,env,stdio:'inherit',windowsHide:true});c.on('error',reject);c.on('exit',code=>code===0?resolve():reject(new Error(`${command}: ${code}`)));}); } catch(error) { for(const log of ['ffbuild/config.log','config.log']) { const file=path.join(cwd,log); if(existsSync(file)) console.error((await readFile(file,'utf8')).split('\n').slice(-100).join('\n')); } throw error; } }
async function source(name){const s=lock[name],dir=path.join(cache,name);if(!existsSync(path.join(dir,'.git')))await run('git',['clone','--depth','1','--branch',s.tag,s.repository,dir]);await run('git',['checkout','--detach',s.commit],dir);return dir;}
const tool=cross?'x86_64-w64-mingw32-':'';
const env={...process.env,PKG_CONFIG_PATH:path.join(prefix,'lib/pkgconfig'),PKG_CONFIG_LIBDIR:path.join(prefix,'lib/pkgconfig'),...(cross?{CC:`${tool}gcc`,CXX:`${tool}g++`,AR:`${tool}ar`,RANLIB:`${tool}ranlib`,CROSS:tool}:{})};
const opus=await source('opus');
await run('cmake',['-S',opus,'-B',path.join(opus,'build'),`-DCMAKE_INSTALL_PREFIX=${prefix}`,'-DCMAKE_BUILD_TYPE=Release','-DBUILD_SHARED_LIBS=OFF','-DOPUS_BUILD_TESTING=OFF',...(cross?['-DCMAKE_SYSTEM_NAME=Windows',`-DCMAKE_C_COMPILER=${tool}gcc`,`-DCMAKE_CXX_COMPILER=${tool}g++`]:[])],root,env);
await run('cmake',['--build',path.join(opus,'build'),'--parallel',jobs],root,env);await run('cmake',['--install',path.join(opus,'build')],root,env);
const vpx=await source('libvpx');
const vpxTarget=cross?'x86_64-win64-gcc':process.platform==='darwin'?(process.arch==='arm64'?'arm64-darwin20-gcc':'x86_64-darwin20-gcc'):'x86_64-linux-gcc';
await run('sh',['./configure',`--prefix=${prefix}`,`--target=${vpxTarget}`,'--disable-examples','--disable-tools','--disable-docs','--disable-unit-tests','--disable-vp9','--disable-shared','--enable-static'],vpx,env);
await run('make',['-j',jobs],vpx,env);await run('make',['install'],vpx,env);
const ffmpeg=await source('ffmpeg');
await run('sh',['./configure','--disable-gpl','--disable-nonfree','--disable-version3','--disable-shared','--enable-static','--disable-autodetect','--disable-doc','--disable-debug','--disable-ffplay','--disable-network','--disable-devices','--enable-indev=lavfi','--enable-libvpx','--enable-libopus','--pkg-config=pkg-config','--pkg-config-flags=--static',`--extra-cflags=-I${prefix}/include`,`--extra-ldflags=-L${prefix}/lib${cross?' -static':''}`,...(cross?['--target-os=mingw32','--arch=x86_64','--enable-cross-compile',`--cross-prefix=${tool}`]:[])],ffmpeg,env);
await run('make',['-j',jobs],ffmpeg,env);
for(const name of ['ffmpeg','ffprobe'])await copyFile(path.join(ffmpeg,name+(cross?'.exe':'')),path.join(output,name+(cross?'.exe':'')));
for(const [name,dir,license]of[['ffmpeg',ffmpeg,'COPYING.LGPLv2.1'],['opus',opus,'COPYING'],['libvpx',vpx,'LICENSE']]){await run('git',['archive','--format=tar.gz','--output',path.join(output,`${name}-source.tar.gz`),'HEAD'],dir);await copyFile(path.join(dir,license),path.join(output,`${name}-LICENSE.txt`));}
await copyFile(path.join(ffmpeg,'ffbuild/config.mak'),path.join(output,'ffmpeg-config.mak'));
await copyFile(new URL(import.meta.url),path.join(output,'build-codecs.mjs'));
await copyFile(path.join(root,'resources/runtimes.json'),path.join(output,'runtime-versions.json'));
await writeFile(path.join(output,'codec-build.json'),JSON.stringify({target,sources:lock,commands:records},null,2));
console.log('Source-built codecs and corresponding source materials:',output);
