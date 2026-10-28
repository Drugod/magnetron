"""Opt-in public-service smoke test. Never part of the deterministic offline gate."""
import argparse,json,os,pathlib,queue,subprocess,sys,tempfile,threading,time
ROOT=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--runtime',required=True)
parser.add_argument('--python',default=sys.executable)
parser.add_argument('--url',default='https://www.youtube.com/watch?v=jNQXAC9IVRw')
args=parser.parse_args()
runtime=pathlib.Path(args.runtime).resolve()
cache=ROOT/'.local-docs';cache.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='video-live-',dir=cache) as temporary:
    root=pathlib.Path(temporary);dest=root/'downloads';dest.mkdir();stage=root/'stage';stage.mkdir()
    kwargs={'creationflags':subprocess.CREATE_NEW_PROCESS_GROUP|subprocess.CREATE_NO_WINDOW} if os.name=='nt' else {'start_new_session':True}
    child=subprocess.Popen([args.python,'-I','-u',str(runtime/'video_worker.py')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,**kwargs)
    events=queue.Queue()
    def read():
        for line in child.stdout:
            try:events.put(json.loads(line))
            except ValueError:pass
        events.put(None)
    threading.Thread(target=read,daemon=True).start()
    completed=False;last_state=None;deadline=time.monotonic()+240
    try:
        child.stdin.write(json.dumps({'uri':args.url,'folder':str(dest.resolve()),'staging':str(stage.resolve()),'quality':360,'down':1024})+'\n');child.stdin.flush()
        while time.monotonic()<deadline:
            try:event=events.get(timeout=1)
            except queue.Empty:continue
            if event is None:break
            if event['kind']=='progress':
                if event.get('total',0)>50*1024*1024:raise RuntimeError('Public fixture exceeded test size limit')
                if event.get('state')!=last_state:
                    last_state=event.get('state');print('Video state:',last_state,flush=True)
            elif event['kind']=='metadata':print('Metadata verified; output files:',len(event.get('files',[])),flush=True)
            elif event['kind']=='completed':completed=True
            elif event['kind']=='error':raise RuntimeError('Public-service download unavailable; no signed media URLs or remote error text were logged')
        child.stdin.close()
        code=child.wait(timeout=10)
        if not completed or code!=0:raise RuntimeError('Public-service video download did not complete')
        files=[p for p in dest.iterdir() if p.is_file()]
        if len(files)!=1 or files[0].stat().st_size==0:raise RuntimeError('Expected one final video')
        print('PASS: public video downloaded and verified; bytes:',files[0].stat().st_size)
    finally:
        if child.poll() is None:
            if os.name=='nt':subprocess.run(['taskkill','/PID',str(child.pid),'/T','/F'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            else:
                import signal
                os.killpg(child.pid,signal.SIGKILL)
            child.wait(timeout=10)
