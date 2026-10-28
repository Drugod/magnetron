"""Sign built native assets with the existing protected Actions key; no publication."""
import json,pathlib,platform,subprocess,sys,shutil
ROOT=pathlib.Path(__file__).resolve().parents[1]
version=json.loads((ROOT/'package.json').read_text(encoding='utf-8'))['version']
if sys.platform=='darwin':
    arch='arm64' if platform.machine()=='arm64' else 'x64'
    jobs=[(f'artifacts/MAGNETRON_{version}_macos-{arch}.app.zip',f'macos-{arch}')]
else:
    jobs=[(f'src-tauri/target/release/bundle/appimage/MAGNETRON_{version}_amd64.AppImage','linux-appimage'),(f'src-tauri/target/release/bundle/deb/MAGNETRON_{version}_amd64.deb','linux-deb')]
for relative,channel in jobs:
    file=ROOT/relative
    if not file.is_file():raise SystemExit('Missing update package: '+relative)
    local=ROOT/'artifacts'/file.name
    if local!=file:shutil.copy2(file,local)
    url=f'https://github.com/Drugod/magnetron/releases/download/v{version}/{file.name}'
    subprocess.run(['node',str(ROOT/'scripts/sign-update.mjs'),'sign-env',str(file),version,url,str(ROOT/f'artifacts/{channel}-update.json')],check=True)
print('Signed native manifests prepared; not published')
