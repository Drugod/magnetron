"""Bundle pinned video dependencies and build a local-file-only FFmpeg."""
import hashlib,json,os,pathlib,platform,shlex,shutil,subprocess,sys,tarfile,zipfile
from importlib.machinery import SourceFileLoader
ROOT=pathlib.Path(__file__).resolve().parents[1]
base=SourceFileLoader('native_prepare',str(ROOT/'scripts/prepare-native.py')).load_module()
LOCK=json.loads((ROOT/'scripts/video-assets.json').read_text(encoding='utf-8'))
FLAGS=['--disable-autodetect','--disable-network','--disable-shared','--enable-static','--disable-doc','--disable-debug','--disable-asm','--disable-x86asm','--disable-everything','--enable-ffmpeg','--enable-ffprobe','--enable-protocol=file,pipe','--enable-demuxer=mov,matroska,mpegts,aac,mp3,rawvideo','--enable-muxer=matroska,webm,mp4,mov,mpegts,adts,mp3','--enable-parser=aac,h264,hevc,av1,vp8,vp9,opus,mpegaudio','--enable-bsf=aac_adtstoasc,av1_frame_merge,av1_frame_split,extract_extradata,h264_mp4toannexb,hevc_mp4toannexb,vp9_superframe']

def main():
    system='windows' if os.name=='nt' else 'macos' if sys.platform=='darwin' else 'linux'
    arch='arm64' if platform.machine().lower() in ('arm64','aarch64') else 'x64'
    key=system+'-'+arch
    runtime=ROOT/('vendor/v2-runtime' if system=='windows' else 'vendor/native/v2')
    if not runtime.is_dir():raise RuntimeError('Prepare the private Python runtime first')
    packages=runtime/'video-packages';packages.mkdir(exist_ok=True)
    tools=runtime/'video-tools';tools.mkdir(exist_ok=True)
    licenses=tools/'licenses';licenses.mkdir(exist_ok=True)
    for item in LOCK['wheels']:
        archive=base.fetch(item)
        base.unzip(archive,packages)
        # Include the complete wheel (source and original licence notices).
        shutil.copy2(archive,licenses/archive.name)
    node_archive=base.fetch(LOCK['node'][key])
    if system=='windows':
        with zipfile.ZipFile(node_archive) as z:
            for filename,output in [('node.exe',tools/'node.exe'),('LICENSE',licenses/'Node-LICENSE.txt')]:
                names=[n for n in z.namelist() if n.endswith('/'+filename) and n.count('/')==1]
                if len(names)!=1:raise ValueError('Unexpected Node archive layout')
                output.write_bytes(z.read(names[0]))
    else:
        with tarfile.open(node_archive) as tar:
            for filename,output in [('/bin/node',tools/'node'),('/LICENSE',licenses/'Node-LICENSE.txt')]:
                members=[m for m in tar.getmembers() if m.name.endswith(filename) and m.isfile() and m.name.count('/')==(2 if filename=='/bin/node' else 1)]
                if len(members)!=1:raise ValueError('Unexpected Node archive layout')
                with tar.extractfile(members[0]) as f:output.write_bytes(f.read())
                if output.name=='node':output.chmod(0o755)
    source=ROOT/'third_party/archives/ffmpeg-9.0.2.tar.xz'
    with source.open('rb') as f:
        if hashlib.file_digest(f,'sha256').hexdigest()!=LOCK['ffmpeg']['sha256']:raise ValueError('Bundled FFmpeg source hash mismatch')
    shutil.copy2(source,licenses/source.name)
    build=ROOT/'vendor/video-build'/key;build.mkdir(parents=True,exist_ok=True)
    stamp=hashlib.sha256((json.dumps(FLAGS)+source.name+LOCK['ffmpeg']['sha256']+key).encode()).hexdigest()
    marker=build/'build-key'
    exe='.exe' if system=='windows' else ''
    if not marker.is_file() or marker.read_text()!=stamp or not all((build/('ffmpeg-9.0.2/'+name+exe)).is_file() for name in ('ffmpeg','ffprobe')):
        with tarfile.open(source) as tar:tar.extractall(build,filter='data')
        work=build/'ffmpeg-9.0.2'
        flags=FLAGS+(['--target-os=mingw32','--arch=x86_64','--extra-ldflags=-static'] if system=='windows' else [])
        script='set -eu\n./configure '+shlex.join(flags)+'\nmake -j'+str(min(os.cpu_count() or 2,4))+' ffmpeg ffprobe\n'
        # Actual newlines, never a shell command constructed from downloaded metadata.
        if system=='windows':
            bash=pathlib.Path('C:/msys64/usr/bin/bash.exe')
            if not bash.is_file():raise RuntimeError('Build requires MSYS2 at C:/msys64')
            env=dict(os.environ,MSYSTEM='MINGW64',CHERE_INVOKING='1')
            compiler='export PATH=/mingw64/bin:/usr/bin:$PATH\n'
            subprocess.run([str(bash),'--noprofile','--norc','-c',compiler+script],cwd=work,env=env,check=True)
        else:subprocess.run(['sh','-c',script],cwd=work,check=True)
        marker.write_text(stamp)
    work=build/'ffmpeg-9.0.2'
    for name in ('ffmpeg','ffprobe'):
        shutil.copy2(work/(name+exe),tools/(name+exe))
        subprocess.run([str(tools/(name+exe)),'-version'],check=True,stdout=subprocess.DEVNULL)
    for name in ('COPYING.LGPLv2.1','LICENSE.md'):
        shutil.copy2(work/name,licenses/('FFmpeg-'+name))
    (licenses/'FFmpeg-build.json').write_text(json.dumps(dict(source=LOCK['ffmpeg'],configure=FLAGS,platform=key),indent=2)+'\n',encoding='utf-8')
    shutil.copy2(ROOT/'src-tauri/v2/video_worker.py',runtime/'video_worker.py')
    print('Pinned YouTube tools prepared for '+key)
if __name__=='__main__':main()
