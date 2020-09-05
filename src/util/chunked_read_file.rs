use std::fs::File;
use std::future::Future;
use std::io::{Read, Seek};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{cmp, io};

use actix_web::error::{BlockingError, Error, ErrorInternalServerError};
use actix_web::web;
use futures_util::{
    future::{FutureExt, LocalBoxFuture},
    stream::Stream,
};
use web::Bytes;

fn handle_error(err: BlockingError<io::Error>) -> Error {
    match err {
        BlockingError::Error(err) => err.into(),
        BlockingError::Canceled => ErrorInternalServerError("Unexpected error"),
    }
}

pub struct ChunkedReadFile {
    pub size: u64,
    pub offset: u64,
    pub file: Option<File>,
    pub fut: Option<LocalBoxFuture<'static, Result<(File, Bytes), BlockingError<io::Error>>>>,
    pub counter: u64,
}

impl Stream for ChunkedReadFile {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(ref mut fut) = self.fut {
            return match Pin::new(fut).poll(cx) {
                Poll::Ready(Ok((file, bytes))) => {
                    self.fut.take();
                    self.file = Some(file);
                    self.offset += bytes.len() as u64;
                    self.counter += bytes.len() as u64;
                    Poll::Ready(Some(Ok(bytes)))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Some(Err(handle_error(e)))),
                Poll::Pending => Poll::Pending,
            };
        }

        let size = self.size;
        let offset = self.offset;
        let counter = self.counter;

        if size == counter {
            Poll::Ready(None)
        } else {
            let mut file = self.file.take().expect("Use after completion");
            self.fut = Some(
                web::block(move || {
                    let max_bytes: usize;
                    max_bytes = cmp::min(size.saturating_sub(counter), 65_536) as usize;
                    let mut buf = Vec::with_capacity(max_bytes);
                    file.seek(io::SeekFrom::Start(offset))?;
                    let nbytes = file.by_ref().take(max_bytes as u64).read_to_end(&mut buf)?;
                    if nbytes == 0 {
                        return Err(io::ErrorKind::UnexpectedEof.into());
                    }
                    Ok((file, Bytes::from(buf)))
                })
                .boxed_local(),
            );
            self.poll_next(cx)
        }
    }
}
