// Private keys must stay outside the repository and distributed artifacts.
import fs from 'node:fs';import crypto from 'node:crypto';
const [mode,...args]=process.argv.slice(2);
if(mode==='generate'){
 const [file]=args;if(!file)throw Error('generate <private-key-path>');
 const {privateKey,publicKey}=crypto.generateKeyPairSync('ed25519');
 fs.writeFileSync(file,privateKey.export({format:'pem',type:'pkcs8'}),{flag:'wx',mode:0o600});
 console.log('MAGNETRON_UPDATE_PUBLIC_KEY='+publicKey.export({format:'der',type:'spki'}).subarray(-32).toString('hex'));
}else if(mode==='sign'){
 const [key,file,version,url,out]=args;if(!out)throw Error('sign <key> <installer> <version> <https-url> <manifest-output>');
 if(!/^\d+\.\d+\.\d+$/.test(version)||new URL(url).protocol!=='https:')throw Error('Invalid release');
 const data=fs.readFileSync(file),payload=JSON.stringify({version,url,sha256:crypto.createHash('sha256').update(data).digest('hex'),size:data.length});
 const signature=crypto.sign(null,Buffer.from(payload),fs.readFileSync(key)).toString('hex');fs.writeFileSync(out,JSON.stringify({payload,signature}));
}else throw Error('Use generate or sign');
