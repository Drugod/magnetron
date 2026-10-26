"""Regression tests for POSIX crash containment using owned synthetic processes."""
import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import time
import unittest

GUARD = pathlib.Path(__file__).resolve().parents[1] / 'src-tauri/v2/engine_guard.py'
ENGINE = "import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print(p.pid,flush=True); time.sleep(60)"

def running(pid):
    status = subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True,check=False).stdout.strip()
    return bool(status) and not status.startswith('Z')

def wait_stopped(pid):
    limit=time.monotonic()+7
    while running(pid) and time.monotonic()<limit: time.sleep(.05)
    return not running(pid)

def cleanup(group):
    try: os.killpg(group,signal.SIGKILL)
    except ProcessLookupError: pass

@unittest.skipUnless(os.name=='posix','POSIX process groups only')
class GuardTests(unittest.TestCase):
    def test_pipe_close_stops_engine_and_descendant_but_not_unrelated_process(self):
        engine=subprocess.Popen([sys.executable,'-c',ENGINE],start_new_session=True,stdout=subprocess.PIPE,text=True)
        unrelated=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'],start_new_session=True)
        guard=None
        try:
            descendant=int(engine.stdout.readline())
            guard=subprocess.Popen([sys.executable,'-I',str(GUARD),str(engine.pid)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,start_new_session=True)
            self.assertEqual(guard.stdout.read(1),b'R')
            guard.stdin.close();self.assertEqual(guard.wait(timeout=7),0)
            self.assertTrue(wait_stopped(engine.pid));self.assertTrue(wait_stopped(descendant))
            self.assertIsNone(unrelated.poll())
        finally:
            cleanup(engine.pid);engine.wait(timeout=5)
            cleanup(unrelated.pid);unrelated.wait(timeout=5)
            if guard and guard.poll() is None: guard.kill();guard.wait()
            if engine.stdout:engine.stdout.close()
            if guard and guard.stdout:guard.stdout.close()

    def test_sigkill_of_parent_closes_lifetime_pipe(self):
        helper = """import subprocess,sys,time
engine=subprocess.Popen([sys.executable,'-c',sys.argv[2]],start_new_session=True,stdout=subprocess.PIPE,text=True)
child=int(engine.stdout.readline())
guard=subprocess.Popen([sys.executable,'-I',sys.argv[1],str(engine.pid)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,start_new_session=True)
assert guard.stdout.read(1)==b'R'
print(engine.pid,child,guard.pid,flush=True)
time.sleep(60)
"""
        parent=subprocess.Popen([sys.executable,'-c',helper,str(GUARD),ENGINE],stdout=subprocess.PIPE,text=True,start_new_session=True)
        engine_pid=guard_pid=None
        try:
            engine_pid,child,guard_pid=map(int,parent.stdout.readline().split())
            parent.kill();parent.wait(timeout=5)
            self.assertTrue(wait_stopped(engine_pid));self.assertTrue(wait_stopped(child));self.assertTrue(wait_stopped(guard_pid))
        finally:
            if parent.poll() is None:parent.kill();parent.wait()
            if engine_pid:cleanup(engine_pid)
            if guard_pid:cleanup(guard_pid)
            if parent.stdout:parent.stdout.close()

    def test_refuses_its_own_process_group(self):
        result=subprocess.run([sys.executable,'-I',str(GUARD),str(os.getpgrp())],input=b'',capture_output=True,timeout=5)
        self.assertEqual(result.returncode,2)
        self.assertEqual(result.stdout,b'')

if __name__=='__main__':unittest.main()
