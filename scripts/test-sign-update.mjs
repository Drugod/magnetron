import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { signUpdate } from './sign-update.mjs';
test('signed manifest binds version, official URL, exact size and installer digest', () => {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'magnetron-sign-test-'));
  try {
    const file=path.join(dir,'fixture.exe');fs.writeFileSync(file,'synthetic installer');
    const {privateKey,publicKey}=crypto.generateKeyPairSync('ed25519');
    const pem=privateKey.export({format:'pem',type:'pkcs8'});
    const hex=publicKey.export({format:'der',type:'spki'}).subarray(-32).toString('hex');
    const url='https://github.com/Drugod/magnetron/releases/download/v0.8.1/MAGNETRON_0.8.1_x64-setup.exe';
    const e=JSON.parse(signUpdate(pem,file,'0.8.1',url,hex));
    assert(crypto.verify(null,Buffer.from(e.payload),publicKey,Buffer.from(e.signature,'hex')));
    assert(!crypto.verify(null,Buffer.from(e.payload+' '),publicKey,Buffer.from(e.signature,'hex')));
    const release=JSON.parse(e.payload);
    assert.equal(release.size,fs.statSync(file).size);
    assert.equal(release.sha256,crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'));
    assert.throws(()=>signUpdate(pem,file,'0.8.1',url,'00'.repeat(32)));
    assert.throws(()=>signUpdate(pem,file,'0.8.1','https://example.com/setup.exe',hex));
    assert.throws(()=>signUpdate(pem,file,'0.08.1',url,hex));
    fs.writeFileSync(file,'');assert.throws(()=>signUpdate(pem,file,'0.8.1',url,hex));
  } finally { fs.rmSync(dir,{recursive:true,force:true}); }
});
