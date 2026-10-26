// Private keys stay outside Git and distributed artifacts. Never log key material.
import fs from 'node:fs';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
export function signUpdate(key, file, version, url, publicHex) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) throw Error('Invalid version');
  const expected = `https://github.com/Drugod/magnetron/releases/download/v${version}/MAGNETRON_${version}_x64-setup.exe`;
  if (url !== expected) throw Error('Installer URL must belong to the official versioned Windows release');
  const privateKey = crypto.createPrivateKey(key);
  if (privateKey.asymmetricKeyType !== 'ed25519') throw Error('Expected Ed25519 key');
  const publicKey = crypto.createPublicKey(privateKey);
  const derived = publicKey.export({format:'der',type:'spki'}).subarray(-32).toString('hex');
  if (publicHex && derived !== publicHex.trim()) throw Error('Signing key does not match the key trusted by the application');
  const data = fs.readFileSync(file);
  if (!data.length || data.length > 512 * 1024 * 1024) throw Error('Invalid installer size');
  const payload = JSON.stringify({version,url,sha256:crypto.createHash('sha256').update(data).digest('hex'),size:data.length});
  const signature = crypto.sign(null,Buffer.from(payload),privateKey).toString('hex');
  if (!crypto.verify(null,Buffer.from(payload),publicKey,Buffer.from(signature,'hex'))) throw Error('Signature self-check failed');
  return JSON.stringify({payload,signature});
}
function main() {
  const [mode,...args] = process.argv.slice(2);
  if (mode === 'generate') {
    const [file] = args; if (!file) throw Error('generate <private-key-path>');
    const {privateKey,publicKey} = crypto.generateKeyPairSync('ed25519');
    fs.writeFileSync(file,privateKey.export({format:'pem',type:'pkcs8'}),{flag:'wx',mode:0o600});
    console.log('MAGNETRON_UPDATE_PUBLIC_KEY='+publicKey.export({format:'der',type:'spki'}).subarray(-32).toString('hex'));
  } else if (mode === 'sign' || mode === 'sign-env') {
    const key = mode === 'sign' ? fs.readFileSync(args.shift()) : process.env.MAGNETRON_UPDATE_PRIVATE_KEY;
    if (!key) throw Error('Missing private signing key');
    const [file,version,url,out] = args;
    if (!out) throw Error('sign[-env] [key-file] <installer> <version> <https-url> <manifest-output>');
    const publicHex = fs.readFileSync(new URL('../src-tauri/update-public-key.txt',import.meta.url),'utf8');
    fs.writeFileSync(out,signUpdate(key,file,version,url,publicHex));
  } else throw Error('Use generate, sign or sign-env');
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
