"""Verify every retained upstream file and reconstruct the two reviewed patches."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[1]/'third_party'
records=json.loads((ROOT/'PATCHES.json').read_text(encoding='utf-8'))
allowed={'librqbit-utp':{'Cargo.toml'},'glib':{'src/variant_iter.rs'}}
for record in records:
    root=ROOT/record['name']
    for relative,digest in record['original_files'].items():
        file=root/relative
        if relative not in allowed[record['name']]:
            assert hashlib.sha256(file.read_bytes()).hexdigest()==digest,str(file)
    actual={str(p.relative_to(root)).replace('\\','/') for p in root.rglob('*') if p.is_file()}
    assert actual==set(record['original_files']),record['name']+' unexpected/missing files'
    for relative in allowed[record['name']]:
        original=ROOT/'original'/('librqbit-utp-Cargo.toml' if record['name']=='librqbit-utp' else 'glib-variant_iter.rs')
        assert hashlib.sha256(original.read_bytes()).hexdigest()==record['original_files'][relative]
        expected=original.read_text(encoding='utf-8')
        if record['name']=='librqbit-utp':
            expected=expected.replace('[dependencies.ringbuf]\nversion = "0.4.7"','[dependencies.ringbuf]\nversion = "0.5.2"')
        else:
            expected=expected.replace('let p: *mut libc::c_char = std::ptr::null_mut();','let mut p: *mut libc::c_char = std::ptr::null_mut();',1).replace('                &p,\n','                &mut p,\n',1)
        assert (root/relative).read_text(encoding='utf-8')==expected,str(root/relative)+' differs from reviewed patch'
print('Reviewed dependency patches and all retained upstream source hashes verified')
