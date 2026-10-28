"""Hash native packages for CI artifacts; does not publish releases."""
import hashlib,pathlib,json,platform,subprocess,sys
root=pathlib.Path(__file__).resolve().parent.parent
version=json.loads((root/'package.json').read_text(encoding='utf-8'))['version']
if sys.platform=='darwin':
    arch='arm64' if platform.machine()=='arm64' else 'x64'
    archive=root/f'artifacts/MAGNETRON_{version}_macos-{arch}.app.zip'
    subprocess.run(['ditto','-c','-k','--keepParent',str(root/'src-tauri/target/release/bundle/macos/MAGNETRON.app'),str(archive)],check=True)
files=[]
for pattern in ['src-tauri/target/release/bundle/deb/*.deb','src-tauri/target/release/bundle/appimage/*.AppImage','src-tauri/target/release/bundle/dmg/*.dmg','artifacts/MAGNETRON-macOS-app.zip','artifacts/MAGNETRON_*_macos-*.app.zip']:
    files.extend(root.glob(pattern))
if not files:raise SystemExit('No native packages were produced')
lines=[]
for path in sorted(files):
    with path.open('rb') as source:digest=hashlib.file_digest(source,'sha256').hexdigest()
    lines.append(digest+' *'+path.name)
(root/'artifacts').mkdir(exist_ok=True)
(root/'artifacts/SHA256SUMS-native.txt').write_text('\n'.join(lines)+'\n')
print('Hashed',len(files),'native package(s)')
