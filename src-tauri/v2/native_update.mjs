// Private, detached Unix installer. Verifies signature and bytes again before replacement.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
export function verifyPackage(manifest, bytes, publicHex, platform, installed) {
  const envelope=JSON.parse(manifest);
  const key=crypto.createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),Buffer.from(publicHex,'hex')]),format:'der',type:'spki'});
  if(!crypto.verify(null,Buffer.from(envelope.payload),key,Buffer.from(envelope.signature,'hex')))throw Error('Invalid update signature');
  const release=JSON.parse(envelope.payload);
  const version=value=>{
    if(!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(value))throw Error('Invalid update version');
    return value.split('.').map(Number);
  };
  const newer=(a,b)=>{for(let i=0;i<3;i++){if(a[i]!==b[i])return a[i]>b[i];}return false;};
  if(!newer(version(release.version),version(installed)))throw Error('Update would not advance installed version');
  const suffix={'linux-appimage':'amd64.AppImage','linux-deb':'amd64.deb','macos-x64':'macos-x64.app.zip','macos-arm64':'macos-arm64.app.zip'}[platform];
  if(!suffix || release.url!==`https://github.com/Drugod/magnetron/releases/download/v${release.version}/MAGNETRON_${release.version}_${suffix}`)throw Error('Wrong update platform or origin');
  if(!bytes.length || bytes.length>512*1024*1024 || bytes.length!==release.size || crypto.createHash('sha256').update(bytes).digest('hex')!==release.sha256)throw Error('Update bytes differ from signed manifest');
  return release;
}
export function regular(file) {
  const stat=fs.lstatSync(file);
  if(!stat.isFile() || stat.isSymbolicLink())throw Error('Update target must be a regular file');
}
export function replaceFile(source,target,backup) {
  regular(source);regular(target);
  if(fs.existsSync(backup))throw Error('Backup already exists');
  const temporary=target+'.new-'+crypto.randomBytes(12).toString('hex');
  try {
    fs.copyFileSync(source,temporary,fs.constants.COPYFILE_EXCL);fs.chmodSync(temporary,0o755);
    const fd=fs.openSync(temporary,'r');try{fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
    // Keep the launch path present throughout replacement, including a crash
    // between backup creation and commit. Fall back to copy on FAT/exFAT.
    try{fs.linkSync(target,backup);}catch(error){
      if(error.code==='EEXIST')throw error;
      fs.copyFileSync(target,backup,fs.constants.COPYFILE_EXCL);
    }
    fs.renameSync(temporary,target);
  } finally {if(fs.existsSync(temporary))fs.unlinkSync(temporary);}
}
export async function install(request, {healthTimeoutMs=90000} = {}) {
  const {root,platform,target,installed,pid,publicKey}=request;
  if(!path.isAbsolute(root)||!path.isAbsolute(target)||!Number.isSafeInteger(pid)||pid<2)throw Error('Invalid installer request');
  const packagePath=path.join(root,'installer.exe');
  regular(packagePath);regular(path.join(root,'manifest.json'));
  const release=verifyPackage(fs.readFileSync(path.join(root,'manifest.json'),'utf8'),fs.readFileSync(packagePath),publicKey,platform,installed);
  const nonce=crypto.randomBytes(16).toString('hex');
  const health=path.join(root,'health-'+nonce);
  const arg='--magnetron-update-health='+nonce;
  const backup=target+'.previous-'+nonce;
  let previousBackup;
  try{
    const journal=path.join(root,'last-install.json');
    if(fs.statSync(journal).size<8192)previousBackup=JSON.parse(fs.readFileSync(journal,'utf8')).previous;
  }catch{}
  const launch=()=>{
    const command=platform.startsWith('macos-')?'/usr/bin/open':target;
    const args=platform.startsWith('macos-')?['-n',target,'--args',arg]:[arg];
    const child=spawn(command,args,{detached:true,stdio:'ignore',env:{...process.env,APPIMAGE:undefined,APPDIR:undefined,LD_LIBRARY_PATH:undefined}});
    child.on('error',()=>{});child.unref();
  };
  // Wait until MAGNETRON has stopped its engines and exited.
  const exitDeadline=Date.now()+120000;
  while(true){
    try{process.kill(pid,0);}catch(error){if(error.code==='ESRCH')break;throw error;}
    if(Date.now()>exitDeadline)throw Error('Application did not exit');
    await pause(100);
  }
  let replaced=false;
  try {
    if(platform==='linux-appimage') {
      if(!target.endsWith('.AppImage'))throw Error('Invalid AppImage target');
      replaceFile(packagePath,target,backup);replaced=true;
    } else if(platform==='linux-deb') {
      // The operating system owns authorisation; MAGNETRON never stores a password.
      const result=spawnSync('/usr/bin/pkexec',['/usr/bin/dpkg','-i',packagePath],{stdio:'ignore',timeout:300000});
      if(result.status!==0)throw Error('Package installation was declined or failed');
    } else {
      if(!target.endsWith('/MAGNETRON.app')||fs.lstatSync(target).isSymbolicLink())throw Error('Invalid application bundle');
      const temporary=path.join(path.dirname(target),'.MAGNETRON-update-'+nonce);
      fs.mkdirSync(temporary,{mode:0o700});
      try {
        const unpack=spawnSync('/usr/bin/ditto',['-x','-k',packagePath,temporary],{stdio:'ignore',timeout:120000});
        if(unpack.status!==0)throw Error('Could not unpack signed application');
        const app=path.join(temporary,'MAGNETRON.app');
        if(!fs.lstatSync(app).isDirectory()||fs.lstatSync(app).isSymbolicLink()||!fs.existsSync(path.join(app,'Contents/MacOS/magnet')))throw Error('Invalid application archive');
        fs.renameSync(target,backup);
        try{fs.renameSync(app,target);replaced=true;}catch(error){fs.renameSync(backup,target);throw error;}
      } finally {fs.rmSync(temporary,{recursive:true,force:true});}
    }
    launch();
    const deadline=Date.now()+healthTimeoutMs;
    while(Date.now()<deadline){
      if(fs.existsSync(health)&&fs.readFileSync(health,'utf8')===release.version){
        fs.unlinkSync(health);
        // Preserve one previous installation for manual recovery, never user data.
        fs.writeFileSync(path.join(root,'last-install.json'),JSON.stringify({version:release.version,previous:replaced?backup:null}),{mode:0o600});
        if(typeof previousBackup==='string' && previousBackup.startsWith(target+'.previous-') && /^[a-f0-9]{32}$/.test(previousBackup.slice((target+'.previous-').length))){
          try{if(!fs.lstatSync(previousBackup).isSymbolicLink())fs.rmSync(previousBackup,{recursive:true,force:true});}catch{}
        }
        return;
      }
      await pause(250);
    }
    throw Error('Updated application did not confirm startup');
  } catch(error) {
    if(replaced){
      const failed=target+'.failed-'+nonce;
      fs.renameSync(target,failed);
      fs.renameSync(backup,target);
    }
    fs.writeFileSync(path.join(root,'install-error.txt'),'The update could not be completed. Check the installation before trying again.',{mode:0o600});
    launch();
    throw error;
  }
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  try{await install(JSON.parse(fs.readFileSync(process.argv[2],'utf8')));}catch{process.exitCode=1;}
}
