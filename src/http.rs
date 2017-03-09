// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Low-level HTTP utilities.

use hyper::client::{Body, RequestBuilder, Response};
use hyper::header::{Header, Headers, HeaderFormat};

use super::{ApiError, ApiResult, Session};
use super::auth::Method as AuthMethod;
use super::identity::protocol;


/// Request builder with authentication.
///
/// Essentially copies the interface of hyper::client::RequestBuilder.
#[allow(missing_debug_implementations)]
pub struct AuthenticatedRequestBuilder<'a, A: AuthMethod + 'a> {
    parent: &'a Session<A>,
    inner: RequestBuilder<'a>
}


impl<'a, Auth: AuthMethod> AuthenticatedRequestBuilder<'a, Auth> {
    /// Wrap a request builder.
    pub fn new(inner: RequestBuilder<'a>, session: &'a Session<Auth>)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder {
            parent: session,
            inner: inner
        }
    }

    /// Send this request.
    pub fn send(self) -> ApiResult<Response> {
        let resp = try!(self.send_unchecked());
        if resp.status.is_success() {
            Ok(resp)
        } else {
            Err(ApiError::HttpError(resp.status, resp))
        }
    }

    /// Send this request without checking on status code.
    pub fn send_unchecked(self) -> ApiResult<Response> {
        let token = try!(self.parent.auth_token());
        let hdr = protocol::AuthTokenHeader(token.into());
        self.inner.header(hdr).send().map_err(From::from)
    }

    /// Add body to the request.
    pub fn body<B: Into<Body<'a>>>(self, body: B)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder {
            inner: self.inner.body(body),
            .. self
        }
    }

    /// Add additional headers to the request.
    pub fn headers(self, headers: Headers)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder {
            inner: self.inner.headers(headers),
            .. self
        }
    }

    /// Add an individual header to the request.
    ///
    /// Note that X-Auth-Token is always overwritten with a token in use.
    pub fn header<H: Header + HeaderFormat>(self, header: H)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder {
            inner: self.inner.header(header),
            .. self
        }
    }
}
