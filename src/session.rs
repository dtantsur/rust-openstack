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
    auth: Auth,
    client: Client,
    cached_token: utils::ValueCache<Auth::TokenType>,
    default_region: Option<String>
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
            auth: auth_method,
            client: utils::http_client(),
            cached_token: utils::ValueCache::new(None),
            default_region: None
        }
    }

    /// Create a new session with a given authentication plugin and region.
    pub fn new_with_region(auth_method: Auth, region: String) -> Session<Auth> {
        Session {
            auth: auth_method,
            client: utils::http_client(),
            cached_token: utils::ValueCache::new(None),
            default_region: Some(region)
        }
    }

    /// Get a clone of the authentication token.
    pub fn auth_token(&self) -> ApiResult<Auth::TokenType> {
        try!(self.refresh_token());
        Ok(self.cached_token.get().unwrap())
    }

    /// Get a reference to the authentication method in use.
    pub fn auth_method(&self) -> &Auth {
        &self.auth
    }

    /// Get a default (usually public) endpoint URL.
    #[inline]
    pub fn get_default_endpoint<S1: Into<String>>(&self, service_type: S1)
            -> ApiResult<Url> {
        self.auth.get_endpoint(service_type.into(),
                                      None,
                                      self.default_region.clone(),
                                      &self)
    }

    /// Get an endpoint URL.
    pub fn get_endpoint<S1, S2>(&self, service_type: S1,
                                endpoint_interface: S2) -> ApiResult<Url>
            where S1: Into<String>, S2: Into<String> {
        self.auth.get_endpoint(service_type.into(),
                                      Some(endpoint_interface.into()),
                                      self.default_region.clone(),
                                      &self)
    }

    /// A wrapper for HTTP request.
    pub fn request<U: IntoUrl>(&'a self, method: Method, url: U)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder {
            parent: self,
            inner: self.client.request(method, url)
        }
    }

    fn refresh_token(&self) -> ApiResult<()> {
        self.cached_token.ensure_value(|| {
            self.auth.get_token(&self.client)
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
    cached_endpoint: utils::ValueCache<Url>
}

impl<'a, Auth: AuthMethod + 'a, S: ServiceType> ServiceApi<'a, Auth, S> {
    /// Create a new API instance using the given session.
    pub fn new(session: &'a Session<Auth>) -> ServiceApi<'a, Auth, S> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: None,
            cached_endpoint: utils::ValueCache::new(None)
        }
    }

    /// Create a new API instance using the given session.
    pub fn new_with_endpoint<S1>(session: &'a Session<Auth>,
                                 endpoint_interface: S1)
            -> ServiceApi<'a, Auth, S> where S1: Into<String> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: Some(endpoint_interface.into()),
            cached_endpoint: utils::ValueCache::new(None)
        }
    }

    /// Get the root endpoint with or without the major version.
    ///
    /// The resulting endpoint is cached on the current ServiceApi object.
    pub fn get_root_endpoint(&self, include_version: bool) -> ApiResult<Url> {
        try!(self.cached_endpoint.ensure_value(|| {
            match self.endpoint_interface {
                Some(ref s) => self.session.get_endpoint(S::catalog_type(),
                                                         s.clone()),
                None => self.session.get_default_endpoint(S::catalog_type())
            }
        }));

        let endpoint = self.cached_endpoint.get().unwrap();
        if include_version {
            if let Some(suffix) = S::version_suffix() {
                if !endpoint.path().ends_with(suffix) {
                    return endpoint.join(suffix).map_err(From::from);
                }
            }
        }

        Ok(endpoint)
    }

    /// Get an endpoint with version suffix and given path appended.
    pub fn get_endpoint(&self, path: &str) -> ApiResult<Url> {
        let endpoint = try!(self.get_root_endpoint(true));
        endpoint.join(path).map_err(From::from)
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
    use super::super::auth::{Identity, Method, Token, NoAuth, SimpleToken};
    use super::super::auth::identity::IdentityAuthMethod;
    use super::super::utils;

    pub fn new_with_params<Auth: Method>(auth: Auth, cli: hyper::Client,
                                         token: Auth::TokenType,
                                         region: Option<&str>)
            -> Session<Auth> {
        Session {
            auth: auth,
            client: cli,
            cached_token: utils::ValueCache::new(Some(token)),
            default_region: region.map(From::from)
        }
    }

    pub fn new_session(token: &str) -> Session<NoAuth> {
        let token = SimpleToken(String::from(token));
        new_with_params(NoAuth::new("http://127.0.0.1/").unwrap(),
                        utils::http_client(), token, None)
    }

    fn session_with_identity(region: Option<&str>)
            -> Session<IdentityAuthMethod> {
        let id = Identity::new("http://127.0.2.1").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockCatalog::default());
        let token = SimpleToken(String::from("abcdef"));
        Session {
            auth: id,
            client: cli,
            cached_token: utils::ValueCache::new(Some(token)),
            default_region: region.map(From::from)
        }
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

    // Copied from keystone API reference.
    const EXAMPLE_CATALOG_RESPONSE: &'static str = r#"
    {
        "catalog": [
            {
                "endpoints": [
                    {
                        "id": "39dc322ce86c4111b4f06c2eeae0841b",
                        "interface": "public",
                        "region": "RegionOne",
                        "url": "http://localhost:5000"
                    },
                    {
                        "id": "ec642f27474842e78bf059f6c48f4e99",
                        "interface": "internal",
                        "region": "RegionOne",
                        "url": "http://localhost:5000"
                    },
                    {
                        "id": "c609fc430175452290b62a4242e8a7e8",
                        "interface": "admin",
                        "region": "RegionOne",
                        "url": "http://localhost:35357"
                    }
                ],
                "id": "4363ae44bdf34a3981fde3b823cb9aa2",
                "type": "identity",
                "name": "keystone"
            }
        ],
        "links": {
            "self": "https://example.com/identity/v3/catalog",
            "previous": null,
            "next": null
        }
    }"#;

    mock_connector!(MockCatalog {
        "http://127.0.2.1" => String::from("HTTP/1.1 200 OK\r\n\
                                            Server: Mock.Mock\r\n\
                                            X-Subject-Token: abcdef\r\n
                                            \r\n") + EXAMPLE_CATALOG_RESPONSE
    });

    #[test]
    fn test_session_new() {
        let s = new_session("foo");
        let token = s.auth_token().unwrap();
        assert_eq!(token.value(), "foo");
        assert!(token.expires_at().is_none());
    }

    #[test]
    fn test_session_request() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = Session {
            auth: NoAuth::new("http://127.0.0.1/").unwrap(),
            client: cli,
            cached_token: utils::ValueCache::new(None),
            default_region: None
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
            auth: NoAuth::new("http://127.0.0.2/").unwrap(),
            client: cli,
            cached_token: utils::ValueCache::new(None),
            default_region: None
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
            auth: NoAuth::new("http://127.0.0.2/").unwrap(),
            client: cli,
            cached_token: utils::ValueCache::new(None),
            default_region: None
        };

        let mut resp = s.request(hyper::Post, "http://127.0.0.2/")
            .body("body").header(ContentLength(4u64)).send_unchecked()
            .unwrap();

        assert_eq!(resp.status, StatusCode::NotFound);

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }

    #[test]
    fn test_session_get_endpoint_no_region() {
        let session = session_with_identity(None);

        let e1 = session.get_default_endpoint("identity").unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:5000/");
        let e2 = session.get_endpoint("identity", "admin").unwrap();
        assert_eq!(&e2.to_string(), "http://localhost:35357/");

        match session.get_default_endpoint("foo").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "foo"),
            other => panic!("Unexpected {}", other)
        };

        match session.get_endpoint("identity", "unknown").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };
    }

    #[test]
    fn test_session_get_endpoint_with_region() {
        let session = session_with_identity(Some("RegionOne"));

        let e1 = session.get_endpoint("identity", "admin").unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:35357/");
    }

    #[test]
    fn test_session_get_endpoint_with_region_fail() {
        let session = session_with_identity(Some("unknown"));

        match session.get_default_endpoint("identity").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };

        match session.get_endpoint("identity", "public").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };
    }
}
