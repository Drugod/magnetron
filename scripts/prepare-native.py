"""Prepare pinned Linux/macOS engines. Run on the target OS with Python 3.12+."""
import argparse, hashlib, json, os, pathlib, platform, plistlib, shutil, subprocess, sys, tarfile, urllib.parse, urllib.request, zipfile
ROOT=pathlib.Path(__file__).resolve().parent.parent
VENDOR=ROOT/'vendor'
LOCK=json.loads((ROOT/'scripts/native-assets.json').read_text())
def run(*args,**kwargs):
    return subprocess.run([str(a) for a in args],check=True,**kwargs)
def reset(path):
    path=path.resolve()
    if path==VENDOR or VENDOR not in path.parents:raise ValueError('Refusing to reset outside vendor')
    if path.exists():shutil.rmtree(path)
    path.mkdir(parents=True)
def fetch(asset):
    url=asset['url'];parsed=urllib.parse.urlsplit(url)
    if parsed.scheme!='https' or len(asset['sha256'])!=64:raise ValueError('Invalid asset lock')
    cache=VENDOR/'native-cache';cache.mkdir(parents=True,exist_ok=True)
    target=cache/urllib.parse.unquote(pathlib.PurePosixPath(parsed.path).name)
    def digest():
        with target.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
    if not target.exists():
        req=urllib.request.Request(url,headers={'User-Agent':'MAGNETRON-native-build'})
        with urllib.request.urlopen(req,timeout=180) as response,target.open('xb') as out:
            size=0
            while chunk:=response.read(1024*1024):
                size+=len(chunk)
                if size>512*1024*1024:raise ValueError('Asset too large')
                out.write(chunk)
    if digest()!=asset['sha256'].lower():raise ValueError('SHA-256 mismatch: '+target.name)
    return target
def untar(archive,destination):
    destination.mkdir(parents=True,exist_ok=True)
    with tarfile.open(archive,'r:gz') as source:
        members=source.getmembers()
        if len(members)>100000 or sum(m.size for m in members)>2*1024**3:raise ValueError('Archive exceeds limits')
        source.extractall(destination,filter='data')
def unzip(archive,destination):
    destination.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(archive) as source:
        infos=source.infolist()
        if len(infos)>100000 or sum(i.file_size for i in infos)>2*1024**3:raise ValueError('ZIP exceeds limits')
        for info in infos:
            p=pathlib.PurePosixPath(info.filename)
            if p.is_absolute() or '..' in p.parts or '\\' in info.filename or ':' in info.filename or (info.external_attr>>16)&0o170000==0o120000:raise ValueError('Unsafe ZIP path')
        source.extractall(destination)
def check_lock():
    for key in ('linux-x64','macos-x64','macos-arm64'):
        for component in ('jackett','amule','python','wheel'):
            a=LOCK[key][component]
            if urllib.parse.urlsplit(a['url']).scheme!='https' or len(a['sha256'])!=64 or any(c not in '0123456789abcdef' for c in a['sha256'].lower()):raise ValueError('Invalid locked asset')
    print('Native asset lock: OK')
def check_daemon_version(daemon):
    result=subprocess.run([str(daemon),'--version'],capture_output=True,text=True,timeout=15)
    output=result.stdout+'\n'+result.stderr
    # wxWidgets returns -1 (255 on Unix) after the successful version-only path.
    if result.returncode not in (0,255) or not any(line.strip().startswith('aMuleD 3.1.0 compiled with') for line in output.splitlines()):
        raise RuntimeError('aMuled version check failed: '+output)
    print(output.strip())
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check-lock',action='store_true');args=parser.parse_args();check_lock()
    if args.check_lock:return
    os.chdir(ROOT)
    system='macos' if sys.platform=='darwin' else 'linux' if sys.platform=='linux' else None
    arch={'x86_64':'x64','AMD64':'x64','arm64':'arm64','aarch64':'arm64'}.get(platform.machine())
    key=f'{system}-{arch}'
    if key not in LOCK:raise SystemExit('Unsupported native target: '+key)
    assets=LOCK[key];native=VENDOR/'native';reset(native)
    stage=VENDOR/'native-stage';reset(stage)
    untar(fetch(assets['jackett']),stage/'jackett')
    engines=list((stage/'jackett').rglob('jackett'))
    engines=[p for p in engines if p.is_file()]
    if len(engines)!=1:raise RuntimeError('Expected one Jackett executable')
    shutil.copytree(engines[0].parent,native/'engine',symlinks=True)
    if system=='linux':
        untar(fetch(assets['amule']),stage/'amule')
        daemons=[p for p in (stage/'amule').rglob('amuled') if p.is_file()]
        if len(daemons)!=1:raise RuntimeError('Expected one aMuled executable')
        (native/'amule/bin').mkdir(parents=True);shutil.copy2(daemons[0],native/'amule/bin/amuled')
    else:
        result=subprocess.check_output(['hdiutil','attach','-readonly','-nobrowse','-plist',str(fetch(assets['amule']))])
        mounts=[pathlib.Path(e['mount-point']) for e in plistlib.loads(result)['system-entities'] if 'mount-point' in e]
        if len(mounts)!=1:raise RuntimeError('Expected one mounted aMule volume')
        mount=mounts[0]
        try:
            daemons=[p for p in mount.rglob('amuled') if p.parent.name=='MacOS' and p.is_file()]
            if len(daemons)!=1:raise RuntimeError('Expected one bundled aMuled executable')
            shutil.copytree(daemons[0].parents[2],native/'amule/aMule.app',symlinks=True)
        finally:run('hdiutil','detach',mount)
    untar(fetch(assets['python']),stage/'python')
    if not (stage/'python/python/bin/python3').exists():raise RuntimeError('Unexpected Python standalone layout')
    shutil.move(stage/'python/python',native/'v2')
    python=native/'v2/bin/python3'
    site=native/'v2/lib/python3.13/site-packages'
    unzip(fetch(assets['wheel']),site)
    shutil.copy2(ROOT/'src-tauri/v2/worker.py',native/'v2/worker.py')
    shutil.copy2(ROOT/'docs/licenses/libtorrent-2.1.1-LICENSE.txt',native/'v2/LICENSE-libtorrent.txt')
    licenses=native/'licenses';licenses.mkdir()
    shutil.copy2(fetch(LOCK['sources']['amule']),licenses/'aMule-3.1.0-source.tar.gz')
    shutil.copy2(fetch(LOCK['sources']['jackett']),licenses/'Jackett-v0.24.2900-source.zip')
    unzip(licenses/'Jackett-v0.24.2900-source.zip',stage/'jackett-source')
    texts=list((stage/'jackett-source').glob('*/LICENSE'))
    if len(texts)!=1:raise RuntimeError('Missing Jackett license')
    shutil.copy2(texts[0],licenses/'Jackett-LICENSE.txt')
    untar(licenses/'aMule-3.1.0-source.tar.gz',stage/'amule-source')
    originals=[p for p in (stage/'amule-source').glob('*/*') if p.is_file() and p.name.lower() in ('copying','license','license.md')]
    if not originals:raise RuntimeError('Missing aMule license')
    for p in originals:shutil.copy2(p,licenses/('aMule-'+p.name))
    host=subprocess.check_output(['rustc','-vV'],text=True).split('host: ')[1].splitlines()[0]
    metadata=ROOT/'artifacts/licenses/metadata.json';metadata.parent.mkdir(parents=True,exist_ok=True)
    with metadata.open('w') as out:run('cargo','metadata','--manifest-path','src-tauri/Cargo.toml','--locked','--format-version','1','--filter-platform',host,stdout=out)
    run('node','scripts/generate-notices.mjs',metadata)
    shutil.copy2(ROOT/'docs/licenses/librqbit-LICENSE.txt',ROOT/'artifacts/licenses/librqbit-LICENSE.txt')
    # Fail before packaging if a target runtime or daemon cannot load its libraries.
    run(python,'-I','-c','import libtorrent,ssl; print(libtorrent.version)')
    daemon=native/('amule/aMule.app/Contents/MacOS/amuled' if system=='macos' else 'amule/bin/amuled')
    check_daemon_version(daemon)
    print('Native engines prepared for '+key)
if __name__=='__main__':main()
