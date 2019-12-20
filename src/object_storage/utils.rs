// Copyright 2020 Dmitry Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Utilities for Object Storage API, mainly around inter-library compatibility.

use std::pin::Pin;
use std::task;

use futures::io::{AsyncRead, Error as IoError, ErrorKind as IoErrorKind};
use futures::stream::TryStreamExt;
use reqwest::{Body, Response};
use tokio::io::AsyncRead as TokioAsyncRead;
use tokio_util::codec;

/// Convert an object implementing AsyncRead to a reqwest Body.
#[inline]
pub fn async_read_to_body(read: impl AsyncRead + Send + Sync + 'static) -> Body {
    let stream = codec::FramedRead::new(AsyncReadCompatWrapper(read), codec::BytesCodec::new())
        .map_ok(|b| b.freeze());
    Body::wrap_stream(stream)
}

/// Convert a response to an object implementing AsyncRead.
#[inline]
pub fn body_to_async_read(mut resp: Response) -> impl AsyncRead + Send + Sync + 'static {
    resp.bytes_stream()
        .map_err(|orig| {
            let kind = if orig.is_timeout() {
                IoErrorKind::TimedOut
            } else {
                IoErrorKind::Other
            };
            IoError::new(kind, orig)
        })
        .into_async_read()
}

// A compatibility wrapper between two incompatible but identical AsyncRead implementations.
//
// That's why we cannot have nice things..
//
// TODO(dtantsur): kill this with fire when Tokio and futures start agreeing on AsyncRead.
#[derive(Debug)]
struct AsyncReadCompatWrapper<T>(T);

impl<T> TokioAsyncRead for AsyncReadCompatWrapper<T>
where
    T: AsyncRead,
{
    #[allow(unsafe_code)]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut task::Context,
        buf: &mut [u8],
    ) -> task::Poll<Result<usize, IoError>> {
        // Safety: the inner field is only ever used in this context.
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        AsyncRead::poll_read(inner, cx, buf)
    }
}
