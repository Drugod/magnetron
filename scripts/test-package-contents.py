"""Check distributed resources without reading or printing profile contents."""
import hashlib,pathlib,sys
root=pathlib.Path(sys.argv[1]).resolve()
required=[
 'licenses/MAGNETRON-GPL-3.0.txt','licenses/librqbit-LICENSE.txt','licenses/THIRD-PARTY-NOTICES.txt',
 'v2/worker.py','v2/engine_guard.py','v2/video_worker.py',
 'v2/video-tools/licenses/Node-LICENSE.txt','v2/video-tools/licenses/ffmpeg-9.0.2.tar.xz',
 'v2/video-tools/licenses/FFmpeg-COPYING.LGPLv2.1','v2/video-tools/licenses/FFmpeg-build.json',
 'v2/video-packages/yt_dlp/__init__.py','v2/video-packages/yt_dlp_ejs/__init__.py',
]
if sys.platform=='win32': required+=['engine/JackettConsole.exe','amule/bin/amuled.exe','v2/python.exe','licenses/aMule-3.1.0-source.zip','licenses/Jackett-v0.24.2900-source.zip']
else: required+=['engine/jackett','v2/bin/python3','licenses/aMule-3.1.0-source.tar.gz','licenses/Jackett-v0.24.2900-source.zip', 'amule/aMule.app/Contents/MacOS/amuled' if sys.platform=='darwin' else 'amule/bin/amuled']
required += ['v2/video-tools/'+n+('.exe' if sys.platform=='win32' else '') for n in ('node','ffmpeg','ffprobe')]
for relative in required:
    p=root/relative
    if not p.is_file() or p.stat().st_size==0:raise SystemExit('Required packaged resource missing: '+relative)
for p in root.rglob('*'):
    if not p.is_file():continue
    name=p.name.lower()
    if name in {'update.private.pem','.env','downloads.json','serverconfig.json','amule.conf','magnet.sqlite'} or name.endswith(('.private.pem','.pfx','.p12')) or name.startswith(('magnet.sqlite-','downloads.json.')):
        raise SystemExit('Private profile/signing material must not be packaged: '+str(p.relative_to(root)))
# The shipped guard must match reviewed application source, not a stale copy.
source=pathlib.Path(__file__).resolve().parents[1]/'src-tauri/v2/engine_guard.py'
if hashlib.sha256((root/'v2/engine_guard.py').read_bytes()).digest()!=hashlib.sha256(source.read_bytes()).digest():raise SystemExit('Packaged process guard differs from reviewed source')
if hashlib.sha256((root/'v2/video_worker.py').read_bytes()).digest()!=hashlib.sha256((source.parent/'video_worker.py').read_bytes()).digest():raise SystemExit('Packaged video worker differs from reviewed source')
print('PASS: packaged engines, runtimes, notices, corresponding engine source and guard; no known private profile/signing filenames')
