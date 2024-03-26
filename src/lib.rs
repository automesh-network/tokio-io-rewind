//! This crate provides a `Rewind` struct that wraps any type that implements
//! the `AsyncRead` and/or `AsyncWrite` traits, allowing for the prepending of
//! bytes before the actual read happens. This is particularly useful in scenarios
//! where you've read bytes from a stream but need to "put them back" to be read
//! again, effectively allowing you to rewind the stream.
//!
//! # Examples
//!
//! Basic usage of `Rewind` with an `AsyncRead` implementor (`tokio::io::Cursor`):
//!
//! ```
//! use bytes::Bytes;
//! use tokio::io::AsyncReadExt; // for read_to_end
//! use rewind::Rewind;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new Rewind instance with a Cursor wrapped inside.
//!     let mut rw = Rewind::new(tokio::io::Cursor::new(b"world".to_vec()));
//!
//!     // Prepend "hello " to the stream.
//!     rw.rewind(Bytes::from_static(b"hello "));
//!
//!     // Read all bytes from the rewinded stream.
//!     let mut buf = Vec::new();
//!     rw.read_to_end(&mut buf).await.unwrap();
//!
//!     assert_eq!(buf, b"hello world");
//! }
//! ```
//!
//! This module also supports asynchronous write operations if the underlying type
//! implements `AsyncWrite`.
//!
//! # Features
//!
//! - `Rewind::new(inner)`: Create a new `Rewind` instance with no pre-buffered bytes.
//! - `Rewind::new_buffered(inner, pre)`: Create a new `Rewind` instance with an initial buffer of bytes to prepend.
//! - `Rewind::rewind(pre)`: Prepend bytes to the current buffer. If there's already a buffer, the new bytes are prepended before the existing ones.
//! - `Rewind::into_inner()`: Consumes the `Rewind`, returning the inner type and any un-read pre-buffered bytes.
//!
//! `Rewind` can be especially useful in protocols or situations where a piece of data is read to determine what comes next,
//! but where that data also needs to be part of the eventual input stream.

use bytes::{Buf, Bytes, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite};

/// Wraps an `AsyncRead` and/or `AsyncWrite` implementor, allowing bytes to be prepended to the stream.
///
/// This is useful for situations where bytes are read from a stream to make decisions and then need to be
/// "unread", making them available for future read operations.
pub struct Rewind<T> {
    inner: T,
    pre: Option<Bytes>,
}

impl<T> Rewind<T> {
    /// Creates a new `Rewind` instance without any pre-buffered bytes.
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner type that implements `AsyncRead` and/or `AsyncWrite`.
    pub fn new(inner: T) -> Self {
        Rewind { inner, pre: None }
    }

    /// Creates a new `Rewind` instance with pre-buffered bytes.
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner type that implements `AsyncRead` and/or `AsyncWrite`.
    /// * `pre` - Initial bytes to prepend to the stream.
    pub fn new_buffered(inner: T, pre: Bytes) -> Self {
        Rewind {
            inner,
            pre: Some(pre),
        }
    }

    /// Prepends bytes to the stream. If there are already pre-buffered bytes,
    /// the new bytes are added before the existing ones.
    ///
    /// # Arguments
    ///
    /// * `pre` - Bytes to prepend.
    pub fn rewind(&mut self, pre: Bytes) {
        match self.pre {
            Some(ref mut old_pre) => {
                let mut new_pre = BytesMut::with_capacity(old_pre.len() + pre.len());
                new_pre.extend_from_slice(&pre);
                new_pre.extend_from_slice(old_pre);
                self.pre = Some(new_pre.freeze());
            }
            None => {
                self.pre = Some(pre);
            }
        }
    }

    /// Consumes the `Rewind`, returning the inner type and any un-read pre-buffered bytes.
    pub fn into_inner(self) -> (T, Bytes) {
        (self.inner, self.pre.unwrap_or_default())
    }
}

impl<T> AsRef<T> for Rewind<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> AsMut<T> for Rewind<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> AsyncRead for Rewind<T>
where
    T: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Some(mut pre) = self.pre.take() {
            let copy_len = std::cmp::min(pre.len(), buf.remaining());
            buf.put_slice(&pre[..copy_len]);
            pre.advance(copy_len);
            if !pre.is_empty() {
                self.pre = Some(pre);
            }
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<T> AsyncWrite for Rewind<T>
where
    T: AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_rewind() {
        let mut rw = Rewind::new(std::io::Cursor::new(b"world".to_vec()));
        rw.rewind(Bytes::from_static(b"hello "));
        let mut buf = Vec::new();
        rw.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"hello world");
        let mut buf = Vec::new();
        rw.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"");
    }
}
