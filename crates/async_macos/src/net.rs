//! Unix domain sockets driven by GCD dispatch sources.

use std::future::poll_fn;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::AsyncRead;
use futures::AsyncWrite;

use crate::gcd::AsyncFd;

pub struct UnixListener(AsyncFd);

impl UnixListener {
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let listener = StdUnixListener::bind(path)?;
        Ok(Self(AsyncFd::new(listener.into())?))
    }

    pub async fn accept(&self) -> io::Result<UnixStream> {
        let fd = poll_fn(|cx| self.0.poll_accept(cx)).await?;
        UnixStream::new(fd)
    }
}

/// Clones share one socket, so a reader and a writer can live in different tasks.
#[derive(Clone)]
pub struct UnixStream(Arc<AsyncFd>);

impl UnixStream {
    fn new(fd: OwnedFd) -> io::Result<Self> {
        Ok(Self(Arc::new(AsyncFd::new(fd)?)))
    }

    pub async fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_owned();
        let stream = crate::unblock(move || StdUnixStream::connect(path)).await?;
        Self::new(stream.into())
    }
}

impl AsyncRead for UnixStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        self.0.poll_read(cx, buf)
    }
}

impl AsyncWrite for UnixStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        // SAFETY: shutdown on a socket we own.
        unsafe {
            libc::shutdown(self.0.as_raw_fd(), libc::SHUT_WR);
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use futures::AsyncReadExt as _;
    use futures::AsyncWriteExt as _;
    use futures::executor::block_on;

    use super::*;

    #[test]
    fn accepts_and_echoes() {
        let path = std::env::temp_dir().join(format!("util-net-{}.sock", std::process::id()));
        std::fs::remove_file(&path).ok();
        let listener = UnixListener::bind(&path).unwrap();

        block_on(async {
            let server = async {
                let stream = listener.accept().await.unwrap();
                let (mut reader, mut writer) = (stream.clone(), stream);
                let mut request = Vec::new();
                reader.read_to_end(&mut request).await.unwrap();
                request.reverse();
                writer.write_all(&request).await.unwrap();
                writer.close().await.unwrap();
            };
            let client = async {
                let mut stream = UnixStream::connect(&path).await.unwrap();
                // Larger than the socket buffer, so both sides hit `WouldBlock`.
                let payload = (0..1_000_000_u32).map(|i| i as u8).collect::<Vec<_>>();
                let mut writer = stream.clone();
                let write = async {
                    writer.write_all(&payload).await.unwrap();
                    writer.close().await.unwrap();
                };
                let mut response = Vec::new();
                let read = async { stream.read_to_end(&mut response).await.unwrap() };
                futures::join!(write, read);
                let mut expected = payload;
                expected.reverse();
                assert!(response == expected);
            };
            futures::join!(server, client);
        });
        std::fs::remove_file(&path).ok();
    }
}
