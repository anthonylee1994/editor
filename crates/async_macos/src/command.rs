#![allow(clippy::disallowed_methods, reason = "this is the wrapper the lint points to")]

use std::collections::HashMap;
use std::ffi::CString;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::future::poll_fn;
use std::io;
use std::iter;
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::pin::Pin;
use std::process::ExitStatus;
use std::process::Output;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::AsyncRead;
use futures::AsyncReadExt as _;
use futures::AsyncWrite;

use crate::gcd::AsyncFd;
use crate::gcd::ProcessExit;

pub fn new_command(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}

pub use std::process::Stdio;

#[derive(Debug)]
pub struct Command {
    inner: std::process::Command,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    kill_on_drop: bool,
    disclaim: bool,
}

impl From<std::process::Command> for Command {
    fn from(inner: std::process::Command) -> Self {
        Self { inner, stdin: false, stdout: false, stderr: false, kill_on_drop: false, disclaim: false }
    }
}

unsafe extern "C" {
    // libSystem SPI, also used by Chromium and iTerm2.
    fn responsibility_spawnattrs_setdisclaim(attrs: *mut libc::posix_spawnattr_t, disclaim: libc::c_int)
    -> libc::c_int;
}

/// What a disclaimed child re-execs itself with. Built before fork, so the
/// child allocates nothing.
struct DisclaimedExec {
    program: CString,
    _strings: Vec<CString>,
    argv: Vec<*mut libc::c_char>,
    envp: Vec<*mut libc::c_char>,
    attrs: libc::posix_spawnattr_t,
}

// The pointers only reference the `CString`s owned by the same value.
unsafe impl Send for DisclaimedExec {}
unsafe impl Sync for DisclaimedExec {}

impl DisclaimedExec {
    fn new(command: &std::process::Command) -> io::Result<Self> {
        let program = CString::new(command.get_program().as_bytes())?;
        let mut env: HashMap<OsString, OsString> = std::env::vars_os().collect();
        for (key, value) in command.get_envs() {
            match value {
                Some(value) => env.insert(key.to_owned(), value.to_owned()),
                None => env.remove(key),
            };
        }

        let mut strings = vec![program.clone()];
        for arg in command.get_args() {
            strings.push(CString::new(arg.as_bytes())?);
        }
        let argc = strings.len();
        for (key, value) in env {
            strings.push(CString::new([key.as_bytes(), b"=", value.as_bytes()].concat())?);
        }
        let pointers = |strings: &[CString]| {
            strings.iter().map(|string| string.as_ptr().cast_mut()).chain(iter::once(std::ptr::null_mut())).collect()
        };
        let argv = pointers(&strings[..argc]);
        let envp = pointers(&strings[argc..]);

        let mut attrs = MaybeUninit::uninit();
        let result = unsafe {
            match libc::posix_spawnattr_init(attrs.as_mut_ptr()) {
                0 => {
                    match libc::posix_spawnattr_setflags(attrs.as_mut_ptr(), libc::POSIX_SPAWN_SETEXEC as libc::c_short)
                    {
                        0 => responsibility_spawnattrs_setdisclaim(attrs.as_mut_ptr(), 1),
                        error => error,
                    }
                }
                error => error,
            }
        };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result));
        }
        Ok(Self { program, _strings: strings, argv, envp, attrs: unsafe { attrs.assume_init() } })
    }

    /// Replaces the calling process. Only returns on failure.
    fn exec(&self) {
        unsafe {
            libc::posix_spawnp(
                std::ptr::null_mut(),
                self.program.as_ptr(),
                std::ptr::null(),
                &raw const self.attrs,
                self.argv.as_ptr(),
                self.envp.as_ptr(),
            );
        }
    }
}

impl Command {
    #[inline]
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        std::process::Command::new(program).into()
    }

    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn get_args(&self) -> impl Iterator<Item = &OsStr> {
        self.inner.get_args()
    }

    pub fn env(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env(key, val);
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.envs(vars);
        self
    }

    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env_remove(key);
        self
    }

    pub fn current_dir(&mut self, dir: impl AsRef<Path>) -> &mut Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn stdin(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.stdin = true;
        self.inner.stdin(cfg.into());
        self
    }

    pub fn stdout(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.stdout = true;
        self.inner.stdout(cfg.into());
        self
    }

    pub fn stderr(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.stderr = true;
        self.inner.stderr(cfg.into());
        self
    }

    pub const fn kill_on_drop(&mut self, kill_on_drop: bool) -> &mut Self {
        self.kill_on_drop = kill_on_drop;
        self
    }

    /// Makes the child, not this app, the process macOS holds responsible
    /// for itself and its descendants. Use it for commands whose descendants
    /// may outlive the app (git starting an ssh `ControlPersist` master), which
    /// would otherwise list the app under "Allow in the Background".
    ///
    /// A process can only be disclaimed as it is exec'd, so the forked child
    /// re-execs itself with `POSIX_SPAWN_SETEXEC`. This costs a fork, where
    /// other commands use `posix_spawn`.
    pub const fn disclaim(&mut self) -> &mut Self {
        self.disclaim = true;
        self
    }

    pub fn spawn(&mut self) -> io::Result<Child> {
        if std::mem::take(&mut self.disclaim) {
            let exec = DisclaimedExec::new(&self.inner)?;
            // The hook runs after std set up stdio, the working directory and
            // signals. If the re-exec fails, std's plain exec follows.
            unsafe {
                self.inner.pre_exec(move || {
                    exec.exec();
                    Ok(())
                });
            }
        }
        let mut child = self.inner.spawn()?;
        Ok(Child {
            stdin: child.stdin.take().map(|pipe| pipe_fd(pipe.into()).map(ChildStdin)).transpose()?,
            stdout: child.stdout.take().map(|pipe| pipe_fd(pipe.into()).map(ChildStdout)).transpose()?,
            stderr: child.stderr.take().map(|pipe| pipe_fd(pipe.into()).map(ChildStderr)).transpose()?,
            pid: child.id(),
            exit: ProcessExit::watch(child.id()),
            kill_on_drop: self.kill_on_drop,
        })
    }

    /// Runs the command and collects its output. Unless configured otherwise,
    /// stdin is null and stdout/stderr are captured.
    pub async fn output(&mut self) -> io::Result<Output> {
        if !self.stdin {
            self.inner.stdin(Stdio::null());
        }
        if !self.stdout {
            self.inner.stdout(Stdio::piped());
        }
        if !self.stderr {
            self.inner.stderr(Stdio::piped());
        }
        self.spawn()?.output().await
    }

    pub async fn status(&mut self) -> io::Result<ExitStatus> {
        self.spawn()?.status().await
    }
}

fn pipe_fd(fd: OwnedFd) -> io::Result<Option<AsyncFd>> {
    AsyncFd::new(fd).map(Some)
}

pub struct Child {
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
    pid: u32,
    exit: Arc<ProcessExit>,
    kill_on_drop: bool,
}

impl Child {
    pub const fn id(&self) -> u32 {
        self.pid
    }

    pub fn kill(&mut self) -> io::Result<()> {
        self.exit.signal(libc::SIGKILL, false);
        Ok(())
    }

    /// Kills the child's whole process group; only meaningful when the child
    /// leads its own group.
    pub fn kill_group(&mut self) {
        self.exit.signal(libc::SIGKILL, true);
    }

    pub fn try_status(&mut self) -> io::Result<Option<ExitStatus>> {
        Ok(self.exit.try_status())
    }

    /// Closes stdin and waits for the child to exit.
    pub async fn status(&mut self) -> io::Result<ExitStatus> {
        self.stdin.take();
        Ok(poll_fn(|cx| self.exit.poll_status(cx)).await)
    }

    pub async fn output(mut self) -> io::Result<Output> {
        self.stdin.take();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdout_pipe = self.stdout.take();
        let mut stderr_pipe = self.stderr.take();
        futures::try_join!(
            async {
                match &mut stdout_pipe {
                    Some(pipe) => pipe.read_to_end(&mut stdout).await.map(drop),
                    None => Ok(()),
                }
            },
            async {
                match &mut stderr_pipe {
                    Some(pipe) => pipe.read_to_end(&mut stderr).await.map(drop),
                    None => Ok(()),
                }
            },
        )?;
        let status = self.status().await?;
        Ok(Output { status, stdout, stderr })
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        if self.kill_on_drop {
            self.exit.signal(libc::SIGKILL, false);
        }
    }
}

/// The write end of a child's stdin; closing or dropping it sends EOF.
pub struct ChildStdin(Option<AsyncFd>);
pub struct ChildStdout(Option<AsyncFd>);
pub struct ChildStderr(Option<AsyncFd>);

impl AsyncWrite for ChildStdin {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match &self.0 {
            Some(fd) => fd.poll_write(cx, buf),
            None => Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.take();
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for ChildStdout {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        match &self.0 {
            Some(fd) => fd.poll_read(cx, buf),
            None => Poll::Ready(Ok(0)),
        }
    }
}

impl AsyncRead for ChildStderr {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        match &self.0 {
            Some(fd) => fd.poll_read(cx, buf),
            None => Poll::Ready(Ok(0)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::zombie_processes, reason = "children are reaped by ProcessExit")]
mod tests {
    use futures::AsyncWriteExt as _;
    use futures::executor::block_on;

    use super::*;

    #[test]
    fn output_captures_stdout_and_stderr() {
        let output = block_on(new_command("/bin/sh").args(["-c", "echo out; echo err >&2; exit 2"]).output()).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr, b"err\n");
    }

    #[test]
    fn status_reports_exit_code() {
        let mut command = Command::from(std::process::Command::new("/usr/bin/false"));
        assert_eq!(block_on(command.status()).unwrap().code(), Some(1));
        assert_eq!(command.get_args().count(), 0);
    }

    #[test]
    fn spawn_pipes_stdin_to_stdout() {
        block_on(async {
            let mut child = new_command("/bin/cat").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
            let mut stdin = child.stdin.take().unwrap();
            let payload = vec![7_u8; 300_000];
            let write = async {
                stdin.write_all(&payload).await.unwrap();
                stdin.close().await.unwrap();
                stdin.write(b"x").await.unwrap_err();
            };
            let ((), output) = futures::join!(write, child.output());
            let output = output.unwrap();
            assert!(output.status.success());
            assert!(output.stdout == payload);
        });
    }

    unsafe extern "C" {
        fn responsibility_get_pid_responsible_for_pid(pid: libc::pid_t) -> libc::pid_t;
    }

    #[test]
    fn disclaim_keeps_args_env_stdio_and_working_directory() {
        let dir = std::env::temp_dir().canonicalize().unwrap();
        let output = block_on(
            new_command("sh")
                .args([
                    "-c",
                    r#"printf '%s|%s|%s|%s' "$1" "$ED_TEST_VAR" "${HOME-unset}" "$(pwd -P)""#,
                    "sh",
                    "arg one",
                ])
                .env("ED_TEST_VAR", "value")
                .env_remove("HOME")
                .current_dir(&dir)
                .disclaim()
                .output(),
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), format!("arg one|value|unset|{}", dir.display()));
    }

    #[test]
    fn disclaim_makes_the_child_responsible_for_itself() {
        let responsible = |disclaim: bool| {
            let mut command = new_command("/bin/sleep");
            command.arg("30").kill_on_drop(true);
            if disclaim {
                command.disclaim();
            }
            let child = command.spawn().unwrap();
            let pid = child.id().cast_signed();
            unsafe { responsibility_get_pid_responsible_for_pid(pid) == pid }
        };
        assert!(responsible(true));
        assert!(!responsible(false));
    }

    #[test]
    fn disclaim_reports_a_missing_program() {
        assert!(new_command("/nonexistent/ed-test").disclaim().spawn().is_err());
    }

    #[test]
    fn kill_and_try_status() {
        block_on(async {
            let mut child = new_command("/bin/sleep").arg("30").spawn().unwrap();
            assert!(child.id() > 0);
            assert_eq!(child.try_status().unwrap(), None);
            child.kill().unwrap();
            let status = child.status().await.unwrap();
            assert!(!status.success());
            assert_eq!(child.try_status().unwrap(), Some(status));
        });
    }

    #[test]
    fn kill_on_drop_terminates_the_child() {
        let child = new_command("/bin/sleep").arg("30").kill_on_drop(true).spawn().unwrap();
        let pid = child.id().cast_signed();
        drop(child);
        // The watcher reaps the child, after which the pid no longer exists.
        while unsafe { libc::kill(pid, 0) } == 0 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn kill_group_takes_down_descendants() {
        block_on(async {
            let mut command = std::process::Command::new("/bin/sh");
            command.args(["-c", "sleep 30 & wait"]);
            // SAFETY: setsid is async-signal-safe.
            unsafe {
                command.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
            let mut child = Command::from(command).spawn().unwrap();
            child.kill_group();
            assert!(!child.status().await.unwrap().success());
        });
    }
}
