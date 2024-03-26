use bytes::{Buf, Bytes, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite};

pub struct Rewind<T> {
    inner: T,
    pre: Option<Bytes>,
}

impl<T> Rewind<T> {
    pub fn new(inner: T) -> Self {
        Rewind { inner, pre: None }
    }

    pub fn new_buffered(inner: T, pre: Bytes) -> Self {
        Rewind {
            inner,
            pre: Some(pre),
        }
    }

    pub fn rewind(&mut self, pre: Bytes) {
        match self.pre {
            Some(ref mut old_pre) => {
                let mut new_pre = BytesMut::with_capacity(old_pre.len() + pre.len());
                new_pre.extend_from_slice(old_pre);
                new_pre.extend_from_slice(&pre);
                self.pre = Some(new_pre.freeze());
            }
            None => {
                self.pre = Some(pre);
            }
        }
    }

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
