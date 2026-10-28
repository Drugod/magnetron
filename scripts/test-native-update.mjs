import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawn,spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { signUpdate } from './sign-update.mjs';
import { verifyPackage,replaceFile,install } from '../src-tauri/v2/native_update.mjs';
function fixture(){
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'magnetron-update-'));
  const {privateKey,publicKey}=crypto.generateKeyPairSync('ed25519');
  return {root,key:privateKey.export({type:'pkcs8',format:'pem'}),publicKey:publicKey.export({type:'spki',format:'der'}).subarray(-32).toString('hex')};
}
test('native signatures bind platform, version and bytes; reject tamper and rollback',()=>{
  const f=fixture();try{
    const file=path.join(f.root,'package');fs.writeFileSync(file,'synthetic archive');
    for(const [platform,suffix] of Object.entries({'linux-appimage':'amd64.AppImage','linux-deb':'amd64.deb','macos-x64':'macos-x64.app.zip','macos-arm64':'macos-arm64.app.zip'})){
      const url=`https://github.com/Drugod/magnetron/releases/download/v0.9.0/MAGNETRON_0.9.0_${suffix}`;
      const manifest=signUpdate(f.key,file,'0.9.0',url,f.publicKey),bytes=fs.readFileSync(file);
      assert.equal(verifyPackage(manifest,bytes,f.publicKey,platform,'0.8.1').version,'0.9.0');
      assert.throws(()=>verifyPackage(manifest,bytes,f.publicKey,platform,'0.9.0'));
      assert.throws(()=>verifyPackage(manifest,Buffer.from('tampered'),f.publicKey,platform,'0.8.1'));
      assert.throws(()=>verifyPackage(manifest,bytes,'00'.repeat(32),platform,'0.8.1'));
      assert.throws(()=>verifyPackage(manifest,bytes,f.publicKey,'windows','0.8.1'));
    }
  }finally{fs.rmSync(f.root,{recursive:true,force:true});}
});
test('replacement retains exact old executable and refuses backup collisions',()=>{
 const f=fixture();try{
  const source=path.join(f.root,'source'),target=path.join(f.root,'application'),backup=path.join(f.root,'previous');
  fs.writeFileSync(source,'new');fs.writeFileSync(target,'old');
  replaceFile(source,target,backup);
  assert.equal(fs.readFileSync(target,'utf8'),'new');assert.equal(fs.readFileSync(backup,'utf8'),'old');
  assert.throws(()=>replaceFile(source,target,backup));
  assert.deepEqual(fs.readdirSync(f.root).sort(),['application','previous','source']);
 }finally{fs.rmSync(f.root,{recursive:true,force:true});}
});
test('a failed atomic commit leaves the original launch path usable',()=>{
 const f=fixture();const rename=fs.renameSync;try{
  const source=path.join(f.root,'source'),target=path.join(f.root,'application'),backup=path.join(f.root,'previous');
  fs.writeFileSync(source,'new');fs.writeFileSync(target,'old');
  fs.renameSync=(from,to)=>{if(from.startsWith(target+'.new-'))throw Error('injected commit failure');return rename(from,to);};
  assert.throws(()=>replaceFile(source,target,backup));
  assert.equal(fs.readFileSync(target,'utf8'),'old');assert.equal(fs.readFileSync(backup,'utf8'),'old');
 }finally{fs.renameSync=rename;fs.rmSync(f.root,{recursive:true,force:true});}
});
test('detached installer waits for exit, replaces AppImage, launches and confirms health',{skip:process.platform==='win32',timeout:20000},async()=>{
 const f=fixture();let helper,parent;try{
  const file=path.join(f.root,'installer.exe'),target=path.join(f.root,'MAGNETRON.AppImage');
  fs.writeFileSync(target,'previous installation');
  const program=['#!/usr/bin/env node',`const fs=require('fs'); const token=process.argv.find(a=>a.startsWith('--magnetron-update-health=')).split('=')[1];fs.writeFileSync(${JSON.stringify(f.root)}+'/health-'+token,'0.9.0');`, ''].join(String.fromCharCode(10));
  fs.writeFileSync(file,program);
  const url='https://github.com/Drugod/magnetron/releases/download/v0.9.0/MAGNETRON_0.9.0_amd64.AppImage';
  fs.writeFileSync(path.join(f.root,'manifest.json'),signUpdate(f.key,file,'0.9.0',url,f.publicKey));
  parent=spawn(process.execPath,['-e','setTimeout(()=>{},1000)'],{stdio:'ignore'});
  const request=path.join(f.root,'request.json');
  fs.writeFileSync(request,JSON.stringify({root:f.root,target,platform:'linux-appimage',installed:'0.8.1',pid:parent.pid,publicKey:f.publicKey}));
  helper=spawn(process.execPath,[path.resolve('src-tauri/v2/native_update.mjs'),request],{stdio:'inherit'});
  const [code]=await once(helper,'exit');assert.equal(code,0);
  const journal=JSON.parse(fs.readFileSync(path.join(f.root,'last-install.json')));
  assert.equal(journal.version,'0.9.0');assert.equal(fs.readFileSync(journal.previous,'utf8'),'previous installation');
  assert.equal(fs.readFileSync(target,'utf8'),fs.readFileSync(file,'utf8'));
 }finally{helper?.kill();parent?.kill();fs.rmSync(f.root,{recursive:true,force:true});}
});

test('failed startup restores the previous application and leaves payload data alone',{skip:process.platform==='win32',timeout:20000},async()=>{
 const f=fixture();let parent;try{
  const target=path.join(f.root,'MAGNETRON.AppImage'),file=path.join(f.root,'installer.exe'),marker=path.join(f.root,'old-started');
  const previous=['#!/usr/bin/env node',`require('fs').writeFileSync(${JSON.stringify(marker)},'restored');`, ''].join(String.fromCharCode(10));
  fs.writeFileSync(target,previous,{mode:0o755});fs.writeFileSync(file,'#!/usr/bin/env node'+String.fromCharCode(10)+'process.exit(0);');
  fs.writeFileSync(path.join(f.root,'user-download.txt'),'keep');
  const url='https://github.com/Drugod/magnetron/releases/download/v0.9.0/MAGNETRON_0.9.0_amd64.AppImage';
  fs.writeFileSync(path.join(f.root,'manifest.json'),signUpdate(f.key,file,'0.9.0',url,f.publicKey));
  parent=spawn(process.execPath,['-e','setTimeout(()=>{},100)'],{stdio:'ignore'});
  await assert.rejects(install({root:f.root,target,platform:'linux-appimage',installed:'0.8.1',pid:parent.pid,publicKey:f.publicKey},{healthTimeoutMs:500}));
  assert.equal(fs.readFileSync(target,'utf8'),previous);
  assert.equal(fs.readFileSync(path.join(f.root,'user-download.txt'),'utf8'),'keep');
  for(let i=0;i<40&&!fs.existsSync(marker);i++)await new Promise(r=>setTimeout(r,50));
  assert.equal(fs.readFileSync(marker,'utf8'),'restored');
 }finally{parent?.kill();fs.rmSync(f.root,{recursive:true,force:true});}
});
test('Mac installer replaces a real bundle and obtains LaunchServices startup acknowledgement',{skip:process.platform!=='darwin',timeout:30000},async()=>{
 const f=fixture();let parent,helper;try{
  const target=path.join(f.root,'MAGNETRON.app'),staging=path.join(f.root,'staging'),bundle=path.join(staging,'MAGNETRON.app');
  for(const app of [target,bundle]){
    fs.mkdirSync(path.join(app,'Contents/MacOS'),{recursive:true});
    const plist=`<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>local.magnetron.updatefixture.${crypto.randomBytes(8).toString('hex')}</string><key>CFBundleName</key><string>MAGNETRON</string><key>CFBundleExecutable</key><string>magnet</string><key>CFBundlePackageType</key><string>APPL</string><key>LSBackgroundOnly</key><true/></dict></plist>`;
    fs.writeFileSync(path.join(app,'Contents/Info.plist'),plist);
  }
  fs.writeFileSync(path.join(target,'Contents/MacOS/magnet'),'#!/bin/sh'+String.fromCharCode(10)+'exit 0'+String.fromCharCode(10),{mode:0o755});
  const program=[`#!${process.execPath}`,`const fs=require('fs');const token=process.argv.find(a=>a.startsWith('--magnetron-update-health=')).split('=')[1];fs.writeFileSync(${JSON.stringify(f.root)}+'/health-'+token,'0.9.0');`, ''].join(String.fromCharCode(10));
  fs.writeFileSync(path.join(bundle,'Contents/MacOS/magnet'),program,{mode:0o755});
  const file=path.join(f.root,'installer.exe');
  assert.equal(spawnSync('/usr/bin/ditto',['-c','-k','--keepParent',bundle,file]).status,0);
  const platform=process.arch==='arm64'?'macos-arm64':'macos-x64';
  const url=`https://github.com/Drugod/magnetron/releases/download/v0.9.0/MAGNETRON_0.9.0_${platform}.app.zip`;
  fs.writeFileSync(path.join(f.root,'manifest.json'),signUpdate(f.key,file,'0.9.0',url,f.publicKey));
  parent=spawn(process.execPath,['-e','setTimeout(()=>{},100)'],{stdio:'ignore'});
  const request=path.join(f.root,'request.json');fs.writeFileSync(request,JSON.stringify({root:f.root,target,platform,installed:'0.8.1',pid:parent.pid,publicKey:f.publicKey}));
  helper=spawn(process.execPath,[path.resolve('src-tauri/v2/native_update.mjs'),request],{stdio:'inherit'});
  const [code]=await once(helper,'exit');assert.equal(code,0);
  const journal=JSON.parse(fs.readFileSync(path.join(f.root,'last-install.json')));
  assert.equal(journal.version,'0.9.0');assert(fs.existsSync(journal.previous));
 }finally{helper?.kill();parent?.kill();fs.rmSync(f.root,{recursive:true,force:true});}
});
