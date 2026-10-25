"""Loopback-only BEP52 transfer, collision and release checks; no public torrent."""
import json, pathlib, subprocess, tempfile, time, sys, queue, threading, os
import libtorrent as lt
ROOT=pathlib.Path(__file__).resolve().parent.parent
WORKER=pathlib.Path(os.environ.get('MAGNETRON_TEST_WORKER',str(ROOT/'src-tauri/v2/worker.py')))
def worker(config):
    p=subprocess.Popen([sys.executable,'-I','-u',str(WORKER)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    p.stdin.write(json.dumps(config)+'\n');p.stdin.flush()
    events=queue.Queue()
    def read():
        for line in p.stdout:events.put(json.loads(line))
    threading.Thread(target=read,daemon=True).start()
    return p,events
def wait(p,events,kind,timeout=45):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        try: e=events.get(timeout=.2)
        except queue.Empty:
            if p.poll() is not None:raise AssertionError('Worker exited: '+p.stderr.read())
            continue
        if e['kind']=='error':
            p.wait(timeout=10);raise AssertionError(str(e)+' '+p.stderr.read()[-4000:])
        if e['kind']==kind:return e
    raise AssertionError('Timeout waiting for '+kind)
def main():
  with tempfile.TemporaryDirectory(prefix='magnetron-v2-') as tmp:
    root=pathlib.Path(tmp);seed=root/'seed';seed.mkdir();dest=root/'dest';dest.mkdir();stage=root/'stage';stage.mkdir()
    payload=bytes(range(256))*4096;(seed/'sample.bin').write_bytes(payload)
    fs=lt.file_storage();fs.add_file('sample.bin',len(payload))
    ct=lt.create_torrent(fs,16384,flags=lt.create_torrent.v2_only);lt.set_piece_hashes(ct,str(seed));ti=lt.torrent_info(ct.generate())
    session=lt.session({'listen_interfaces':'127.0.0.1:0','enable_dht':False,'enable_lsd':False,'enable_upnp':False,'enable_natpmp':False,'enable_outgoing_utp':False})
    handle=session.add_torrent({'ti':ti,'save_path':str(seed),'flags':lt.torrent_flags.seed_mode})
    deadline=time.monotonic()+5
    while not session.listen_port() and time.monotonic()<deadline:time.sleep(.05)
    uri=lt.make_magnet_uri(ti)+'&x.pe=127.0.0.1:'+str(session.listen_port())
    config={'uri':uri,'folder':str(dest),'staging':str(stage),'down':0,'up':0}
    # Metadata-only admission must observe a complete seed without writing payload.
    verify_dest=root/'verify';verify_dest.mkdir()
    pv,ev=worker({**config,'folder':str(verify_dest),'staging':str(verify_dest),'verify':True,'test_loopback':True})
    try:
      wait(pv,ev,'verified');assert pv.wait(timeout=10)==0
      assert not any(p.is_file() and p.stat().st_size for p in verify_dest.rglob('*'))
    finally:
      if pv.poll() is None:pv.kill();pv.wait()
    p,events=worker(config)
    try:
      wait(p,events,'completed');assert p.wait(timeout=10)==0
      assert (dest/'sample.bin').read_bytes()==payload
      # Completed worker must release the file before reporting completion.
      (dest/'sample.bin').rename(dest/'renamed.bin');(dest/'renamed.bin').rename(dest/'sample.bin')
      p2,events2=worker(config)
      try:
        e=None
        deadline=time.monotonic()+30
        while time.monotonic()<deadline:
          e=events2.get(timeout=30)
          if e['kind']=='error':break
        assert e['kind']=='error' and 'already exists' in e['error'];p2.wait(timeout=10)
        assert (dest/'sample.bin').read_bytes()==payload
      finally:
        if p2.poll() is None:p2.kill();p2.wait()
      # Corrupt an existing partial and resume; completion requires a hash recheck.
      (dest/'sample.bin').write_bytes(b'X'*len(payload))
      config['resume']=True
      p3,events3=worker(config)
      try:
        wait(p3,events3,'completed');assert p3.wait(timeout=10)==0
        assert (dest/'sample.bin').read_bytes()==payload
      finally:
        if p3.poll() is None:p3.kill();p3.wait()
      (dest/'sample.bin').unlink()
      assert list(dest.iterdir())==[]
      # A reachable peer with metadata but no content must never be admitted.
      (seed/'incomplete.bin').write_bytes(b'private-test'*2048)
      fs2=lt.file_storage();fs2.add_file('incomplete.bin',(seed/'incomplete.bin').stat().st_size)
      ct2=lt.create_torrent(fs2,16384,flags=lt.create_torrent.v2_only);lt.set_piece_hashes(ct2,str(seed));ti2=lt.torrent_info(ct2.generate())
      empty=root/'empty-seed';empty.mkdir()
      session2=lt.session({'listen_interfaces':'127.0.0.1:0','enable_dht':False,'enable_lsd':False,'enable_upnp':False,'enable_natpmp':False})
      h2=session2.add_torrent({'ti':ti2,'save_path':str(empty)})
      while not session2.listen_port():time.sleep(.05)
      negative=lt.make_magnet_uri(ti2)+'&x.pe=127.0.0.1:'+str(session2.listen_port())
      pn,en=worker({**config,'uri':negative,'folder':str(verify_dest),'staging':str(verify_dest),'verify':True,'test_loopback':True})
      try:
        deadline=time.monotonic()+40;rejected=False
        while time.monotonic()<deadline:
          event=en.get(timeout=40)
          assert event['kind']!='verified'
          if event['kind']=='error':rejected=True;break
        assert rejected;assert pn.wait(timeout=10)!=0
      finally:
        if pn.poll() is None:pn.kill();pn.wait()
        session2.remove_torrent(h2);del h2;del session2
      print('PASS: v2 complete-source admission, incomplete-source rejection, transfer, payload hash, collision protection, corrupted resume, file release')
    finally:
      if p.poll() is None:p.kill();p.wait()
      session.remove_torrent(handle);del handle;del session
def seed_mode(folder):
    root=pathlib.Path(folder);(root/'sample.bin').write_bytes(bytes(range(256))*4096)
    fs=lt.file_storage();fs.add_file('sample.bin',1024*1024);ct=lt.create_torrent(fs,16384,flags=lt.create_torrent.v2_only);lt.set_piece_hashes(ct,str(root));ti=lt.torrent_info(ct.generate())
    session=lt.session({'listen_interfaces':'127.0.0.1:0','enable_dht':False,'enable_lsd':False,'enable_upnp':False,'enable_natpmp':False})
    handle=session.add_torrent({'ti':ti,'save_path':str(root),'flags':lt.torrent_flags.seed_mode})
    while not session.listen_port():time.sleep(.05)
    print(lt.make_magnet_uri(ti)+'&x.pe=127.0.0.1:'+str(session.listen_port()),flush=True)
    sys.stdin.readline();session.remove_torrent(handle);del handle;del session
if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='--seed':seed_mode(sys.argv[2])
    else:main()
