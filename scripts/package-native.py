"""Hash native packages for CI artifacts; does not publish releases."""
import hashlib,pathlib
root=pathlib.Path(__file__).resolve().parent.parent
files=[]
for pattern in ['src-tauri/target/release/bundle/deb/*.deb','src-tauri/target/release/bundle/appimage/*.AppImage','src-tauri/target/release/bundle/dmg/*.dmg','artifacts/MAGNETRON-macOS-app.zip']:
    files.extend(root.glob(pattern))
if not files:raise SystemExit('No native packages were produced')
lines=[]
for path in sorted(files):
    with path.open('rb') as source:digest=hashlib.file_digest(source,'sha256').hexdigest()
    lines.append(digest+' *'+path.name)
(root/'artifacts').mkdir(exist_ok=True)
(root/'artifacts/SHA256SUMS-native.txt').write_text('\n'.join(lines)+'\n')
print('Hashed',len(files),'native package(s)')
