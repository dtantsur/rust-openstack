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

//! Session structure definition.
//!
//! The Session object serves as a wrapper around an HTTP(s) client, handling
//! authentication, accessing the service catalog and token refresh.

use std::cell::RefCell;

use hyper::{Client, Url};
use hyper::client::{Body, IntoUrl, RequestBuilder, Response};
use hyper::header::{Header, Headers, HeaderFormat};
use hyper::method::Method;

use super::ApiError;
use super::auth::base::{AuthMethod, AuthTokenHeader};
use super::utils;


/// Request builder with authentication.
///
/// Essentially copies the interface of hyper::client::RequestBuilder.
#[allow(missing_debug_implementations)]
pub struct AuthenticatedRequestBuilder<'a, A: AuthMethod + 'a> {
    parent: &'a Session<A>,
    inner: RequestBuilder<'a>
}

/// An OpenStack API session.
///
/// Owns a token and an underlying client.
#[derive(Debug)]
pub struct Session<A: AuthMethod> {
    auth_method: A,
    client: Client,
    cached_token: RefCell<Option<A::TokenType>>
}

impl<'a, A: AuthMethod> AuthenticatedRequestBuilder<'a, A> {
    /// Send this request.
    pub fn send(self) -> Result<Response, ApiError> {
        let token = try!(self.parent.auth_token());
        let hdr = AuthTokenHeader(token.into());
        self.inner.header(hdr).send().map_err(From::from)
    }

    /// Add body to the request.
    pub fn body<B: Into<Body<'a>>>(self, body: B)
            -> AuthenticatedRequestBuilder<'a, A> {
        AuthenticatedRequestBuilder {
            inner: self.inner.body(body),
            .. self
        }
    }

    /// Add additional headers to the request.
    pub fn headers(self, headers: Headers)
            -> AuthenticatedRequestBuilder<'a, A> {
        AuthenticatedRequestBuilder {
            inner: self.inner.headers(headers),
            .. self
        }
    }

    /// Add an individual header to the request.
    ///
    /// Note that X-Auth-Token is always overwritten with a token in use.
    pub fn header<H: Header + HeaderFormat>(self, header: H)
            -> AuthenticatedRequestBuilder<'a, A> {
        AuthenticatedRequestBuilder {
            inner: self.inner.header(header),
            .. self
        }
    }
}


impl<'a, A: AuthMethod + 'a> Session<A> {
    /// Create a new session with a given authentication plugin.
    pub fn new(auth_method: A) -> Session<A> {
        Session {
            auth_method: auth_method,
            client: utils::http_client(),
            cached_token: RefCell::new(None)
        }
    }

    /// Get a clone of the authentication token.
    pub fn auth_token(&self) -> Result<A::TokenType, ApiError> {
        try!(self.refresh_token());
        Ok(self.cached_token.borrow().clone().unwrap())
    }

    /// Get an endpoint URL.
    pub fn get_endpoint(&self, service_type: &str,
                        endpoint_interface: Option<&str>,
                        region: Option<&str>) -> Result<Url, ApiError> {
        self.auth_method.get_endpoint(service_type, endpoint_interface,
                                      region, &self)
    }

    /// A wrapper for HTTP request.
    pub fn request<U: IntoUrl>(&'a self, method: Method, url: U)
            -> AuthenticatedRequestBuilder<'a, A> {
        AuthenticatedRequestBuilder {
            parent: self,
            inner: self.client.request(method, url)
        }
    }

    // Private and test-only

    #[cfg(test)]
    pub fn new_with_params(auth_method: A, client: Client,
                           token: A::TokenType) -> Session<A> {
        Session {
            auth_method: auth_method,
            client: client,
            cached_token: RefCell::new(Some(token))
        }
    }

    fn refresh_token(&self) -> Result<(), ApiError> {
        let mut cached_token = self.cached_token.borrow_mut();
        if cached_token.is_some() {
            return Ok(())
        }

        // TODO: check expires_at

        let new_token = try!(self.auth_method.get_token(&self.client));
        *cached_token = Some(new_token);
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use std::cell::RefCell;
    use std::io::Read;

    use hyper;
    use hyper::header::ContentLength;

    use super::Session;
    use super::super::auth::base::{AuthToken, NoAuth, SimpleAuthToken};
    use super::super::utils;

    pub fn new_session(token: &str) -> Session<NoAuth> {
        let token = SimpleAuthToken(String::from(token));
        Session::new_with_params(NoAuth::new("http://127.0.0.1/").unwrap(),
                                 utils::http_client(), token)
    }

    mock_connector!(MockHttp {
        "http://127.0.0.1" => "HTTP/1.1 200 OK\r\n\
                               Server: Mock.Mock\r\n\
                               \r\n\
                               {}"
    });

    #[test]
    fn test_session_new() {
        let s = new_session("foo");
        let token = s.auth_token().unwrap();
        assert_eq!(token.value(), "foo");
        assert!(token.expires_at().is_none());
    }

    #[test]
    fn test_session_get_endpoint() {
        let s = new_session("foo");
        let e = s.get_endpoint("foo", None, None).unwrap();
        assert_eq!(&e.to_string(), "http://127.0.0.1/");
    }

    #[test]
    fn test_session_request() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = Session {
            auth_method: NoAuth::new("http://127.0.0.1/").unwrap(),
            client: cli,
            cached_token: RefCell::new(None)
        };

        let mut resp = s.request(hyper::Post, "http://127.0.0.1/")
            .body("body").header(ContentLength(4u64)).send().unwrap();

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }
}
