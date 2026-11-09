//! Child processes with line-based output: the SPT server, spt-additions tasks
//! and the game. Each runs in its own process group so it can be stopped as a whole.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use anyhow::{Context, Result};

use crate::text::strip_ansi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

pub struct ManagedProcess {
    child: Arc<Mutex<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    pid: i32,
    readers: Mutex<Vec<JoinHandle<()>>>,
}

impl ManagedProcess {
    /// Spawns `cmd`; every output line (ANSI stripped) goes to `on_line`.
    pub fn spawn(mut cmd: Command, on_line: impl Fn(Stream, String) + Send + Sync + 'static) -> Result<Self> {
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).process_group(0);
        let prog = cmd.get_program().to_string_lossy().into_owned();
        let mut child = cmd.spawn().with_context(|| format!("starting {prog}"))?;
        let pid = child.id() as i32;
        let stdin = child.stdin.take();
        let on_line = Arc::new(on_line);
        let mut readers = Vec::new();
        if let Some(out) = child.stdout.take() {
            let f = on_line.clone();
            readers.push(std::thread::spawn(move || read_lines(out, |l| f(Stream::Stdout, l))));
        }
        if let Some(err) = child.stderr.take() {
            let f = on_line.clone();
            readers.push(std::thread::spawn(move || read_lines(err, |l| f(Stream::Stderr, l))));
        }
        Ok(Self { child: Arc::new(Mutex::new(child)), stdin: Mutex::new(stdin), pid, readers: Mutex::new(readers) })
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    pub fn write_line(&self, line: &str) -> Result<()> {
        let mut guard = self.stdin.lock().unwrap();
        let stdin = guard.as_mut().context("stdin closed")?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn try_wait(&self) -> Option<ExitStatus> {
        self.child.lock().unwrap().try_wait().ok().flatten()
    }

    /// SIGTERM to the whole process group
    pub fn terminate(&self) {
        unsafe {
            libc::killpg(self.pid, libc::SIGTERM);
        }
    }

    pub fn kill(&self) {
        unsafe {
            libc::killpg(self.pid, libc::SIGKILL);
        }
    }

    /// Blocks until the process exits and its output has been read
    pub fn wait(&self) -> Option<ExitStatus> {
        let status = loop {
            if let Some(s) = self.try_wait() {
                break Some(s);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        let readers: Vec<_> = self.readers.lock().unwrap().drain(..).collect();
        for r in readers {
            let _ = r.join();
        }
        status
    }
}

/// Splits on `\n` and `\r` (progress bars redraw with `\r`), skips empty lines
fn read_lines(r: impl Read, mut f: impl FnMut(String)) {
    let mut reader = BufReader::new(r);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buf);
                for part in text.split('\r') {
                    let line = strip_ansi(part.trim_end_matches('\n'));
                    if !line.trim().is_empty() {
                        f(line);
                    }
                }
            }
        }
    }
}

/// Signals every process in a list (used for the game's wine processes)
pub fn terminate_pids(pids: &[i32]) {
    for &p in pids {
        unsafe {
            libc::kill(p, libc::SIGTERM);
        }
    }
}

/// First executable named `name` on PATH
pub fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
    })
}

/// Starts a detached program (no output capture), e.g. the official launcher
pub fn spawn_detached(mut cmd: Command) -> Result<()> {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    let prog = cmd.get_program().to_string_lossy().into_owned();
    let mut child = cmd.spawn().with_context(|| format!("starting {prog}"))?;
    // Reap it in the background so it does not linger as a zombie
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
