//! A separate pipe-driven watchdog survives an abrupt Unix application exit.
use std::os::unix::process::CommandExt;
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub(crate) struct Job {
    watcher: Child,
}
impl Job {
    pub(crate) fn attach(child: &Child, runtime: &Path) -> Result<Self, String> {
        let watcher = Command::new(crate::platform::python(runtime))
            .arg("-I")
            .arg(runtime.join("engine_guard.py"))
            .arg(child.id().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
        let mut watcher = match watcher {
            Ok(watcher) => watcher,
            Err(e) => {
                terminate_owned_group(child.id());
                return Err(format!("Engine guard could not start: {e}"));
            }
        };
        let mut output = watcher
            .stdout
            .take()
            .ok_or("Missing engine guard handshake")?;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut byte = [0];
            let ok = output.read_exact(&mut byte).is_ok() && byte == [b'R'];
            let _ = tx.send(ok);
        });
        if rx.recv_timeout(Duration::from_secs(5)) != Ok(true) {
            terminate_owned_group(child.id());
            let _ = watcher.kill();
            let _ = watcher.wait();
            return Err("Engine guard handshake failed".into());
        }
        Ok(Self { watcher })
    }
}
// Only called for engine process IDs created in a new group by this application.
fn terminate_owned_group(pid: u32) {
    if pid > 1 && pid <= i32::MAX as u32 {
        unsafe {
            libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // Closing the pipe requests group termination, including descendants.
        self.watcher.stdin.take();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match self.watcher.try_wait() {
                Ok(Some(_)) => return,
                Err(_) => break,
                _ => std::thread::sleep(Duration::from_millis(25)),
            }
        }
        let _ = self.watcher.kill();
        let _ = self.watcher.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dropping_guard_terminates_the_owned_engine() {
        let mut command = Command::new("/bin/sleep");
        command.arg("60").process_group(0);
        let mut child = command.spawn().unwrap();
        let job = Job::attach(&child, &crate::platform::dev_runtime()).unwrap();
        drop(job);
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let stopped = child.try_wait().unwrap().is_some();
        if !stopped {
            let _ = child.kill();
        }
        let _ = child.wait();
        assert!(stopped, "Engine remained alive after its guard was dropped");
    }
}
