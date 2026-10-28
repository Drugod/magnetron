"""Single public YouTube video download; private pinned tools, JSON-only stdio."""
import hashlib,json,os,pathlib,re,shutil,subprocess,sys,threading,time,urllib.parse,unicodedata
STOP=threading.Event()
LOCK=threading.Lock()
RATE=0
NEXT=0.0

def emit(kind,**fields):
    print(json.dumps(dict(kind=kind,**fields),ensure_ascii=True),flush=True)

def canonical(value):
    u=urllib.parse.urlsplit(value)
    ids=urllib.parse.parse_qs(u.query).get('v',[])
    if (u.scheme!='https' or u.netloc!='www.youtube.com' or u.path!='/watch'
        or len(ids)!=1 or not re.fullmatch(r'[A-Za-z0-9_-]{11}',ids[0])):
        raise ValueError('Invalid individual YouTube URL')
    return 'https://www.youtube.com/watch?v='+ids[0],ids[0]

def limited(_self,start,now,count):
    # One aggregate limiter, including separate audio/video and fragments.
    # The downloader calls with a cumulative counter, so track deltas per task.
    global NEXT
    if STOP.is_set():raise InterruptedError('Download stopped')
    previous=getattr(_self,'_magnetron_count',0)
    delta=max(0,count-previous)
    _self._magnetron_count=count
    with LOCK:
        rate=RATE
        if not rate:return
        current=time.monotonic()
        NEXT=max(current,NEXT)+delta/rate
        deadline=NEXT
    while time.monotonic()<deadline:
        if STOP.wait(min(.1,max(0,deadline-time.monotonic()))):raise InterruptedError('Download stopped')
        with LOCK:
            if RATE!=rate:return

def commands():
    global RATE,NEXT
    for line in sys.stdin:
        try:command=json.loads(line)
        except (ValueError,TypeError):continue
        if command.get('op')=='limits':
            with LOCK:
                RATE=max(0,int(command.get('down',0)))*1024;NEXT=time.monotonic()
        elif command.get('op')=='stop':break
    STOP.set()

class Logger:
    def debug(self,_):pass
    def info(self,_):pass
    def warning(self,_):pass
    def error(self,_):pass

def digest(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').digest()

def publish(source,destination,identifier):
    """Exclusive final creation: preserve any unrelated file, even in a race."""
    folder=destination.parent
    if destination.exists():
        if not destination.is_symlink() and destination.is_file() and digest(source)==digest(destination):return
        raise ValueError('A file with this name already exists; it has not been replaced')
    # A unique same-volume file allows an atomic, no-overwrite hard link.
    import tempfile
    temp=None
    try:
        with tempfile.NamedTemporaryFile(prefix='.magnetron-'+identifier+'-',suffix='.part',dir=folder,delete=False) as out:
            temp=pathlib.Path(out.name)
            with source.open('rb') as incoming:
                while chunk:=incoming.read(1024*1024):
                    if STOP.is_set():raise InterruptedError('Download stopped')
                    out.write(chunk)
            out.flush();os.fsync(out.fileno())
        if STOP.is_set():raise InterruptedError('Download stopped')
        os.link(temp,destination)
    finally:
        if temp is not None:temp.unlink(missing_ok=True)

def main():
    global RATE
    request=json.loads(sys.stdin.readline(65537))
    uri,identifier=canonical(request['uri'])
    quality=int(request.get('quality',1080))
    if quality not in (0,360,480,720,1080,1440,2160):raise ValueError('Invalid video quality')
    selected=pathlib.Path(request['folder'])
    folder=selected.resolve(strict=True)
    if selected!=folder:raise ValueError('The download directory changed')
    stage=pathlib.Path(request['staging']).resolve(strict=True)
    if not folder.is_dir() or not stage.is_dir():raise ValueError('Invalid download directory')
    runtime=pathlib.Path(__file__).resolve().parent
    tools=runtime/'video-tools'
    node=tools/('node.exe' if os.name=='nt' else 'node')
    ffmpeg=tools/('ffmpeg.exe' if os.name=='nt' else 'ffmpeg')
    probe=tools/('ffprobe.exe' if os.name=='nt' else 'ffprobe')
    if not all(p.is_file() for p in (node,ffmpeg,probe)):raise ValueError('The bundled video tools are missing')
    # -I ignores user paths/configuration; import only our pinned local wheels.
    sys.path.insert(0,str(runtime/'video-packages'))
    from yt_dlp import YoutubeDL
    from yt_dlp.downloader.common import FileDownloader
    FileDownloader.slow_down=limited
    RATE=max(0,int(request.get('down',0)))*1024
    threading.Thread(target=commands,daemon=True).start()
    last=[0.0]
    def progress(data):
        if STOP.is_set():raise InterruptedError('Download stopped')
        now=time.monotonic()
        if now-last[0]<.4 and data.get('status')!='finished':return
        last[0]=now
        emit('progress',state='downloading',completed=int(data.get('downloaded_bytes') or 0),total=int(data.get('total_bytes') or data.get('total_bytes_estimate') or 0),down=int(data.get('speed') or 0),up=0)
    height='' if quality==0 else f'[height<={quality}]'
    options={
      'format':f'bestvideo{height}+bestaudio/best{height}',
      'outtmpl':str(stage/'payload.%(ext)s'), 'noplaylist':True,
      'quiet':True,'no_warnings':True,'logger':Logger(),
      'progress_hooks':[progress], 'continuedl':True,'overwrites':False,
      'ffmpeg_location':str(tools), 'merge_output_format':'mkv',
      'js_runtimes':{'node':{'path':str(node)}}, 'remote_components':set(),
      'cachedir':False,'socket_timeout':20,'retries':3,'fragment_retries':3,
      'concurrent_fragment_downloads':1,'restrictfilenames':True,
      'allow_unplayable_formats':False,'hls_prefer_native':True,
      'match_filter':lambda info,**_: 'Live streams are not supported' if info.get('is_live') else None,
    }
    with YoutubeDL(options) as ydl:
        info=ydl.extract_info(uri,download=True)
        if not info or info.get('_type') in ('playlist','multi_video') or info.get('id')!=identifier:raise ValueError('An individual video is required')
        source=pathlib.Path(info.get('filepath') or ydl.prepare_filename(info)).resolve(strict=True)
    if source.parent!=stage or source.is_symlink() or source.suffix not in ('.mkv','.mp4','.webm','.3gp','.flv'):
        raise ValueError('Invalid video output')
    if STOP.is_set():raise InterruptedError('Download stopped')
    emit('progress',state='finalizing',completed=source.stat().st_size,total=source.stat().st_size,down=0,up=0)
    result=subprocess.run([str(probe),'-v','error','-show_entries','stream=codec_type','-of','json',str(source)],capture_output=True,timeout=30,check=True)
    streams=json.loads(result.stdout)['streams']
    if not any(s.get('codec_type')=='video' for s in streams):raise ValueError('The downloaded file has no video stream')
    title=str(info.get('title') or 'YouTube')
    title=''.join(c for c in title if unicodedata.category(c) not in ('Cc','Cf','Cs'))
    safe=re.sub(r'[<>:"/\\|?*\x00-\x1f\x7f]','_',title).strip(' .').encode('utf-8')[:180].decode('utf-8','ignore').rstrip(' .') or 'YouTube'
    name=f'{safe} [{identifier}]{source.suffix}'
    emit('metadata',name=name,files=[name],total=source.stat().st_size)
    publish(source,folder/name,identifier)
    emit('completed')

if __name__=='__main__':
    try:main()
    except BaseException as error:
        # Do not expose signed media URLs, cookies, query parameters or remote HTML.
        emit('error',error='YouTube: no se pudo completar la descarga. Comprueba la conexión e intenta reanudar.')
        sys.exit(1)
