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

use std::fmt;
use std::marker::PhantomData;

use hyper::{Client, Get, Url};
use hyper::client::{Body, IntoUrl, RequestBuilder, Response};
use hyper::header::{Header, Headers, HeaderFormat};
use hyper::method::Method;
use serde::Deserialize;
use serde_json;

use super::{ApiError, ApiResult, ServiceType};
use super::auth::Method as AuthMethod;
use super::identity::protocol;
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
pub struct Session<Auth: AuthMethod> {
    auth_method: Auth,
    client: Client,
    cached_token: utils::ValueCache<Auth::TokenType>
}

impl<'a, Auth: AuthMethod> AuthenticatedRequestBuilder<'a, Auth> {
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


impl<'a, Auth: AuthMethod + 'a> Session<Auth> {
    /// Create a new session with a given authentication plugin.
    pub fn new(auth_method: Auth) -> Session<Auth> {
        Session {
            auth_method: auth_method,
            client: utils::http_client(),
            cached_token: utils::ValueCache::new(None)
        }
    }

    /// Get a clone of the authentication token.
    pub fn auth_token(&self) -> ApiResult<Auth::TokenType> {
        try!(self.refresh_token());
        Ok(self.cached_token.get().unwrap())
    }

    /// Get an endpoint URL.
    pub fn get_endpoint(&self, service_type: &str,
                        endpoint_interface: Option<&str>,
                        region: Option<&str>) -> ApiResult<Url> {
        self.auth_method.get_endpoint(service_type, endpoint_interface,
                                      region, &self)
    }

    /// A wrapper for HTTP request.
    pub fn request<U: IntoUrl>(&'a self, method: Method, url: U)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder {
            parent: self,
            inner: self.client.request(method, url)
        }
    }

    // Private and test-only

    #[cfg(test)]
    pub fn new_with_params(auth_method: Auth, client: Client,
                           token: Auth::TokenType) -> Session<Auth> {
        Session {
            auth_method: auth_method,
            client: client,
            cached_token: utils::ValueCache::new(Some(token))
        }
    }

    fn refresh_token(&self) -> ApiResult<()> {
        self.cached_token.ensure_value(|| {
            self.auth_method.get_token(&self.client)
        })
    }
}

/// API version (major, minor).
#[derive(Copy, Clone, Debug)]
pub struct ApiVersion(pub u16, pub Option<u16>);

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(minor) = self.1 {
            write!(f, "{}.{}", self.0, minor)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// Low-level API calls.
#[derive(Debug)]
pub struct ServiceApi<'a, Auth: AuthMethod + 'a, S> {
    session: &'a Session<Auth>,
    service_type: PhantomData<S>,
    endpoint_interface: Option<String>,
    region: Option<String>
}

impl<'a, Auth: AuthMethod + 'a, S: ServiceType> ServiceApi<'a, Auth, S> {
    /// Create a new API instance using the given session.
    pub fn new(session: &'a Session<Auth>) -> ServiceApi<'a, Auth, S> {
        ServiceApi::new_with_endpoint_params(session, None, None)
    }

    /// Create a new API instance using the given session.
    ///
    /// This variant allows passing an endpoint type (defaults to public),
    /// and region (defaults to any).
    pub fn new_with_endpoint_params(session: &'a Session<Auth>,
                                    endpoint_interface: Option<&str>,
                                    region: Option<&str>)
            -> ServiceApi<'a, Auth, S> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: endpoint_interface.map(String::from),
            region: region.map(String::from)
        }
    }

    /// Get an endpoint with version suffix and given path appended.
    pub fn get_endpoint(&self, path: &str) -> ApiResult<Url> {
        let endpoint = try!(self.session.get_endpoint(
                S::catalog_type(),
                self.endpoint_interface.as_ref().map(String::as_str),
                self.region.as_ref().map(String::as_str)));

        let with_version = if let Some(suffix) = S::version_suffix() {
            if endpoint.path().ends_with(suffix) {
                endpoint
            } else {
                try!(endpoint.join(suffix))
            }
        } else {
            endpoint
        };

        with_version.join(path).map_err(From::from)
    }

    /// List entities.
    pub fn list<R: Deserialize>(&self, path: &str) -> ApiResult<R> {
        // TODO: filtering
        let url = try!(self.get_endpoint(path));
        debug!("Listing entities from {}", url);
        let resp = try!(self.session.request(Get, url).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }

    /// Get one entity.
    pub fn get<R: Deserialize, Id: utils::IntoId>(&self, path: &str, id: Id)
            -> ApiResult<R> {
        // Url expects trailing /
        let root_path = if path.ends_with("/") {
            String::from(path)
        } else {
            format!("{}/", path)
        };
        let url = try!(self.get_endpoint(&root_path));
        let url_with_id = try!(url.join(&id.into_id()));
        debug!("Get one entity from {}", url);
        let resp = try!(self.session.request(Get, url_with_id).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }
}

#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use std::io::Read;

    use hyper;
    use hyper::header::ContentLength;
    use hyper::status::StatusCode;

    use super::{ApiError, Session};
    use super::super::auth::{Token, NoAuth, SimpleToken};
    use super::super::utils;

    pub fn new_session(token: &str) -> Session<NoAuth> {
        let token = SimpleToken(String::from(token));
        Session::new_with_params(NoAuth::new("http://127.0.0.1/").unwrap(),
                                 utils::http_client(), token)
    }

    mock_connector!(MockHttp {
        "http://127.0.0.1" => "HTTP/1.1 200 OK\r\n\
                               Server: Mock.Mock\r\n\
                               \r\n\
                               {}"
        "http://127.0.0.2" => "HTTP/1.1 404 NOT FOUND\r\n\
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
            cached_token: utils::ValueCache::new(None)
        };

        let mut resp = s.request(hyper::Post, "http://127.0.0.1/")
            .body("body").header(ContentLength(4u64)).send().unwrap();

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }

    #[test]
    fn test_session_request_error() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = Session {
            auth_method: NoAuth::new("http://127.0.0.2/").unwrap(),
            client: cli,
            cached_token: utils::ValueCache::new(None)
        };

        let err = s.request(hyper::Post, "http://127.0.0.2/")
            .body("body").header(ContentLength(4u64)).send().err().unwrap();

        match err {
            ApiError::HttpError(StatusCode::NotFound, ..) => (),
            other => panic!("Unexpected {}", other)
        }
    }

    #[test]
    fn test_session_request_unchecked_error() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = Session {
            auth_method: NoAuth::new("http://127.0.0.2/").unwrap(),
            client: cli,
            cached_token: utils::ValueCache::new(None)
        };

        let mut resp = s.request(hyper::Post, "http://127.0.0.2/")
            .body("body").header(ContentLength(4u64)).send_unchecked()
            .unwrap();

        assert_eq!(resp.status, StatusCode::NotFound);

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }
}
