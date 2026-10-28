"""Offline video safety and actual bundled mux/probe tests; no public downloads."""
import importlib.util,io,json,os,pathlib,subprocess,sys,tempfile,threading,time,unittest
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
    def test_limiter_can_be_interrupted_without_waiting_for_old_limit(self):
        m.RATE=1;m.NEXT=0
        task=type('Task',(),{})();done=threading.Event()
        def throttle():
            try:m.limited(task,0,0,1024)
            except InterruptedError:done.set()
        thread=threading.Thread(target=throttle);thread.start();time.sleep(.02);m.STOP.set();thread.join(1)
        self.assertFalse(thread.is_alive());self.assertTrue(done.is_set())
    def test_real_bundled_mux_and_probe(self):
        runtime=ROOT/('vendor/v2-runtime' if os.name=='nt' else 'vendor/native/v2')
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
if __name__=='__main__':unittest.main()
