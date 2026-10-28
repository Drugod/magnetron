// Read-only offline validation of the complete, versioned release upload set.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
export async function verifyAssets(root,version,publicHex) {
  if(!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version))throw Error('Invalid release version');
  const channels={'windows':'x64-setup.exe','linux-appimage':'amd64.AppImage','linux-deb':'amd64.deb','macos-x64':'macos-x64.app.zip','macos-arm64':'macos-arm64.app.zip'};
  const key=crypto.createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),Buffer.from(publicHex.trim(),'hex')]),format:'der',type:'spki'});
  const checked=[];
  for(const [channel,suffix] of Object.entries(channels)){
    const name=`MAGNETRON_${version}_${suffix}`,file=path.join(root,name);
    const envelope=JSON.parse(fs.readFileSync(path.join(root,channel+'-update.json'),'utf8'));
    if(!crypto.verify(null,Buffer.from(envelope.payload),key,Buffer.from(envelope.signature,'hex')))throw Error('Invalid '+channel+' signature');
    const release=JSON.parse(envelope.payload);
    if(release.version!==version||release.url!==`https://github.com/Drugod/magnetron/releases/download/v${version}/${name}`)throw Error('Mixed version or channel in '+channel);
    const stat=fs.lstatSync(file);
    if(!stat.isFile()||stat.isSymbolicLink()||stat.size!==release.size)throw Error('Wrong '+channel+' asset size');
    const hash=crypto.createHash('sha256');
    for await(const chunk of fs.createReadStream(file))hash.update(chunk);
    if(hash.digest('hex')!==release.sha256)throw Error('Wrong '+channel+' asset digest');
    checked.push(name);
  }
  for(const extra of [`MAGNETRON-${version}-source.zip`,`MAGNETRON_${version}_aarch64.dmg`,`MAGNETRON_${version}_x64.dmg`]){
    const file=path.join(root,extra);
    if(!fs.statSync(file).isFile()||fs.statSync(file).size===0)throw Error('Missing '+extra);
    checked.push(extra);
  }
  return checked;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  const [root,version]=process.argv.slice(2);
  const key=fs.readFileSync(new URL('../src-tauri/update-public-key.txt',import.meta.url),'utf8');
  const checked=await verifyAssets(root,version,key);
  console.log('PASS:',checked.length,'release packages/source; all five update channels match trusted signatures, versions and exact bytes');
}
