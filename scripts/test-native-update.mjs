import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { signUpdate } from './sign-update.mjs';
import { verifyPackage,replaceFile } from '../src-tauri/v2/native_update.mjs';
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
