"""Offline video safety and actual bundled mux/probe tests; no public downloads."""
import importlib.util,io,json,os,pathlib,subprocess,sys,tempfile,threading,time,unittest,contextlib,shutil
from unittest.mock import patch
ROOT=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('video_worker',ROOT/'src-tauri/v2/video_worker.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class VideoSafety(unittest.TestCase):
    def setUp(self):m.STOP.clear();m.RATE=0
    def test_canonical_and_unsafe_links(self):
        self.assertEqual(m.canonical('https://www.youtube.com/watch?v=abcdefgh_-1')[1],'abcdefgh_-1')
        for uri in ('file:///tmp/a','https://youtube.com.evil.test/watch?v=abcdefgh_-1','https://user@www.youtube.com/watch?v=abcdefgh_-1','https://www.youtube.com/watch?v=abcdefgh_-1&v=abcdefgh_-2'):
            with self.assertRaises(ValueError):m.canonical(uri)
    def test_publish_preserves_collisions_and_resumes_identical_payload(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);source=root/'private';source.write_bytes(b'video payload')
            target=root/'video.mkv';m.publish(source,target,'abcdefgh_-1')
            self.assertEqual(target.read_bytes(),source.read_bytes())
            m.publish(source,target,'abcdefgh_-1')
            target.write_bytes(b'unrelated user data')
            with self.assertRaises(ValueError):m.publish(source,target,'abcdefgh_-1')
            self.assertEqual(target.read_bytes(),b'unrelated user data')
            self.assertEqual(sorted(p.name for p in root.iterdir()),['private','video.mkv'])
    def test_cancel_during_publication_removes_only_owned_temporary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);source=root/'private';source.write_bytes(b'x'*4096)
            other=root/'user.txt';other.write_bytes(b'keep');m.STOP.set()
            with self.assertRaises(InterruptedError):m.publish(source,root/'output.mkv','abcdefgh_-1')
            self.assertEqual(sorted(p.name for p in root.iterdir()),['private','user.txt'])
    def test_recovery_removes_only_the_original_owned_publication_partial(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);journal=root/'publish.json';partial=root/'.magnetron-abcdefgh_-1-fixture.part'
            partial.write_bytes(b'interrupted copy');info=partial.stat()
            record={'folder':str(root),'name':partial.name,'identity':[info.st_dev,info.st_ino]}
            journal.write_text(json.dumps(record),encoding='utf-8')
            m.clean_interrupted_publication(journal,root,'abcdefgh_-1')
            self.assertFalse(partial.exists())
            partial.write_bytes(b'unrelated replacement')
            record['identity']=[-1,-1];journal.write_text(json.dumps(record),encoding='utf-8')
            m.clean_interrupted_publication(journal,root,'abcdefgh_-1')
            self.assertEqual(partial.read_bytes(),b'unrelated replacement')
            outside=root/'keep.txt';outside.write_bytes(b'keep')
            record['name']='../keep.txt';journal.write_text(json.dumps(record),encoding='utf-8')
            m.clean_interrupted_publication(journal,root,'abcdefgh_-1')
            self.assertEqual(outside.read_bytes(),b'keep')
    def test_limiter_can_be_interrupted_without_waiting_for_old_limit(self):
        m.RATE=1;m.NEXT=0
        task=type('Task',(),{})();done=threading.Event()
        def throttle():
            try:m.limited(task,0,0,1024)
            except InterruptedError:done.set()
        thread=threading.Thread(target=throttle);thread.start();time.sleep(.02);m.STOP.set();thread.join(1)
        self.assertFalse(thread.is_alive());self.assertTrue(done.is_set())
    def test_real_bundled_mux_and_probe(self):
        runtime=pathlib.Path(os.environ.get('MAGNETRON_VIDEO_RUNTIME',str(ROOT/('vendor/v2-runtime' if os.name=='nt' else 'vendor/native/v2'))))
        tools=runtime/'video-tools';suffix='.exe' if os.name=='nt' else ''
        self.assertTrue((tools/('ffmpeg'+suffix)).is_file(),'Prepare the bundled video tools first')
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);raw=root/'input.yuv';raw.write_bytes(bytes([80])*16*16+bytes([128])*128)
            output=root/'video.mkv'
            subprocess.run([str(tools/('ffmpeg'+suffix)),'-v','error','-f','rawvideo','-pixel_format','yuv420p','-video_size','16x16','-framerate','1','-i',str(raw),'-c:v','copy','-allow_raw_vfw','1',str(output)],check=True,capture_output=True,timeout=30)
            result=subprocess.run([str(tools/('ffprobe'+suffix)),'-v','error','-show_entries','stream=codec_type','-of','json',str(output)],check=True,capture_output=True,timeout=30)
            self.assertTrue(any(s['codec_type']=='video' for s in json.loads(result.stdout)['streams']))
            node=subprocess.run([str(tools/('node'+suffix)),'--version'],check=True,capture_output=True,timeout=10)
            self.assertEqual(node.stdout.strip(),b'v24.22.1')
            sys.path.insert(0,str(runtime/'video-packages'))
            import yt_dlp,yt_dlp_ejs
            self.assertTrue(yt_dlp.YoutubeDL)
            # Exercise the actual worker pipeline with a controlled extractor;
            # muxing, probing, destination publication and JSON events remain real.
            stage=root/'stage';stage.mkdir();dest=root/'downloads';dest.mkdir()
            request={'uri':'https://www.youtube.com/watch?v=abcdefgh_-1','folder':str(dest.resolve()),'staging':str(stage.resolve()),'quality':720}
            class Extractor:
                def __init__(self,options):self.options=options
                def __enter__(self):return self
                def __exit__(self,*args):pass
                def extract_info(self,uri,download):
                    self.asserted_uri=uri
                    assert '[height<=720]' in self.options['format']
                    assert self.options['noplaylist'] and not self.options['remote_components']
                    final=stage/'payload.mkv';shutil.copyfile(output,final)
                    return {'id':'abcdefgh_-1','title':'Safe / video title','filepath':str(final)}
            events=io.StringIO()
            m.STOP.clear()
            with patch.object(m,'__file__',str(runtime/'video_worker.py')),patch.object(m,'commands',lambda:None),patch.object(m.sys,'stdin',io.StringIO(json.dumps(request)+'\n')),patch.object(yt_dlp,'YoutubeDL',Extractor),contextlib.redirect_stdout(events):
                m.main()
            messages=[json.loads(line) for line in events.getvalue().splitlines()]
            self.assertEqual(messages[-1]['kind'],'completed')
            metadata=next(event for event in messages if event['kind']=='metadata')
            self.assertEqual(len(list(dest.iterdir())),1)
            self.assertEqual((dest/metadata['files'][0]).read_bytes(),output.read_bytes())
            self.assertFalse((stage/'payload.mkv').exists(),'Completed videos must not retain a duplicate payload')
if __name__=='__main__':unittest.main()
