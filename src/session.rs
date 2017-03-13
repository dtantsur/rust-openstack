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

use hyper::{Client, Get, Url};
use hyper::client::IntoUrl;
use hyper::method::Method;
use serde::Deserialize;
use serde_json;

use super::ApiResult;
use super::auth::Method as AuthMethod;
use super::http::AuthenticatedRequestBuilder;
use super::service::{ServiceInfo, ServiceType};
use super::utils;


/// An OpenStack API session.
///
/// The session object serves as a wrapper around an HTTP(s) client, handling
/// authentication, accessing the service catalog and token refresh.
///
/// The session object also owns region and endpoint interface to use.
#[derive(Debug)]
pub struct Session<Auth: AuthMethod> {
    auth: Auth,
    client: Client,
    cached_token: utils::ValueCache<Auth::TokenType>,
    cached_info: utils::MapCache<(&'static str, String), ServiceInfo>,
    region: Option<String>,
    endpoint_interface: String
}


impl<'a, Auth: AuthMethod + 'a> Session<Auth> {
    /// Create a new session with a given authentication plugin.
    ///
    /// The resulting session will use the default endpoint interface (usually,
    /// public) and the first available region.
    pub fn new(auth_method: Auth) -> Session<Auth> {
        let ep = auth_method.default_endpoint_interface();
        Session {
            auth: auth_method,
            client: utils::http_client(),
            cached_token: utils::ValueCache::new(None),
            cached_info: utils::MapCache::new(),
            region: None,
            endpoint_interface: ep
        }
    }

    /// Convert this session into one using the given region.
    pub fn with_region<S: Into<String>>(self, region: S) -> Session<Auth> {
        Session {
            auth: self.auth,
            client: self.client,
            cached_token: self.cached_token,
            // ServiceInfo has to be refreshed
            cached_info: utils::MapCache::new(),
            region: Some(region.into()),
            endpoint_interface: self.endpoint_interface
        }
    }

    /// Convert this session into one using the given endpoint interface.
    pub fn with_endpoint_interface<S: Into<String>>(self,
                                                    endpoint_interface: S)
            -> Session<Auth> {
        Session {
            auth: self.auth,
            client: self.client,
            cached_token: self.cached_token,
            // ServiceInfo has to be refreshed
            cached_info: utils::MapCache::new(),
            region: self.region,
            endpoint_interface: endpoint_interface.into()
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

    /// Get service info for the given service.
    pub fn get_service_info<Srv>(&self) -> ApiResult<ServiceInfo>
            where Srv: ServiceType {
        let key = (Srv::catalog_type(), self.endpoint_interface.clone());

        try!(self.cached_info.ensure_value(key.clone(), |_| {
            self.get_catalog_endpoint(Srv::catalog_type())
                .and_then(|ep| Srv::service_info(ep, self))
        }));

        Ok(self.cached_info.get(&key).unwrap())
    }

    /// Construct and endpoint for the given service.
    pub fn get_endpoint<Srv>(&self, path: &[&str]) -> ApiResult<Url>
            where Srv: ServiceType {
        let info = try!(self.get_service_info::<Srv>());
        Ok(utils::url::extend(info.root_url, path))
    }

    /// Get an endpoint URL from the catalog.
    pub fn get_catalog_endpoint<S>(&self, service_type: S) -> ApiResult<Url>
            where S: Into<String> {
        self.auth.get_endpoint(service_type.into(),
                               Some(self.endpoint_interface.clone()),
                               self.region.clone(),
                               &self)
    }

    /// Make a GET request.
    pub fn http_get<Srv, Res>(&self, path: &[&str]) -> ApiResult<Res>
            where Srv: ServiceType, Res: Deserialize {
        let request = try!(self.request::<Srv>(Get, path));
        let resp = try!(request.send());
        serde_json::from_reader(resp).map_err(From::from)
    }

    /// Make an HTTP request to the given service.
    pub fn request<Srv>(&'a self, method: Method, path: &[&str])
            -> ApiResult<AuthenticatedRequestBuilder<'a, Auth>>
            where Srv: ServiceType {
        let url = try!(self.get_endpoint::<Srv>(path));
        Ok(self.raw_request(method, url))
    }

    /// A wrapper for HTTP request.
    pub fn raw_request<U: IntoUrl>(&'a self, method: Method, url: U)
            -> AuthenticatedRequestBuilder<'a, Auth> {
        AuthenticatedRequestBuilder::new(self.client.request(method, url),
                                         self)
    }

    fn refresh_token(&self) -> ApiResult<()> {
        self.cached_token.ensure_value(|| {
            self.auth.get_token(&self.client)
        })
    }
}

impl<Auth: AuthMethod + Clone> Clone for Session<Auth> {
    fn clone(&self) -> Session<Auth> {
        Session {
            auth: self.auth.clone(),
            client: utils::http_client(),
            cached_token: self.cached_token.clone(),
            cached_info: self.cached_info.clone(),
            region: self.region.clone(),
            endpoint_interface: self.endpoint_interface.clone()
        }
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

    use super::super::ApiError;
    use super::super::auth::{Identity, Method, Token, NoAuth, SimpleToken};
    use super::super::auth::identity::PasswordAuth;
    use super::super::utils;
    use super::Session;

    pub fn new_with_params<Auth: Method>(auth: Auth, cli: hyper::Client,
                                         token: Auth::TokenType,
                                         region: Option<&str>)
            -> Session<Auth> {
        let ep = auth.default_endpoint_interface();
        Session {
            auth: auth,
            client: cli,
            cached_token: utils::ValueCache::new(Some(token)),
            cached_info: utils::MapCache::new(),
            region: region.map(From::from),
            endpoint_interface: ep
        }
    }

    pub fn new_session(token: &str) -> Session<NoAuth> {
        let token = SimpleToken(String::from(token));
        new_with_params(NoAuth::new("http://127.0.0.1/").unwrap(),
                        utils::http_client(), token, None)
    }

    fn session_with_identity() -> Session<PasswordAuth> {
        let id = Identity::new("http://127.0.2.1").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockCatalog::default());
        let token = SimpleToken(String::from("abcdef"));
        let ep = id.default_endpoint_interface();
        Session {
            auth: id,
            client: cli,
            cached_token: utils::ValueCache::new(Some(token)),
            cached_info: utils::MapCache::new(),
            region: None,
            endpoint_interface: ep
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
    fn test_session_raw_request() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = new_with_params(NoAuth::new("http://127.0.0.1/").unwrap(),
                                cli, SimpleToken(String::from("token")), None);

        let mut resp = s.raw_request(hyper::Post, "http://127.0.0.1/")
            .body("body").header(ContentLength(4u64)).send().unwrap();

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }

    #[test]
    fn test_session_raw_request_error() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = new_with_params(NoAuth::new("http://127.0.0.2/").unwrap(),
                                cli, SimpleToken(String::from("token")), None);

        let err = s.raw_request(hyper::Post, "http://127.0.0.2/")
            .body("body").header(ContentLength(4u64)).send().err().unwrap();

        match err {
            ApiError::HttpError(StatusCode::NotFound, ..) => (),
            other => panic!("Unexpected {}", other)
        }
    }

    #[test]
    fn test_session_raw_request_unchecked_error() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = new_with_params(NoAuth::new("http://127.0.0.2/").unwrap(),
                                cli, SimpleToken(String::from("token")), None);

        let mut resp = s.raw_request(hyper::Post, "http://127.0.0.2/")
            .body("body").header(ContentLength(4u64)).send_unchecked()
            .unwrap();

        assert_eq!(resp.status, StatusCode::NotFound);

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }

    #[test]
    fn test_session_get_catalog_endpoint() {
        let session = session_with_identity();

        let e1 = session.get_catalog_endpoint("identity").unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:5000/");

        match session.get_catalog_endpoint("foo").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) => assert_eq!(endp, "foo"),
            other => panic!("Unexpected {}", other)
        };
    }

    #[test]
    fn test_session_get_catalog_endpoint_with_endpoint_interface() {
        let sess1 = session_with_identity().with_endpoint_interface("admin");
        let e2 = sess1.get_catalog_endpoint("identity").unwrap();
        assert_eq!(&e2.to_string(), "http://localhost:35357/");

        let sess2 = session_with_identity().with_endpoint_interface("unknown");
        match sess2.get_catalog_endpoint("identity").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };
    }

    #[test]
    fn test_session_get_catalog_endpoint_with_region() {
        let sess1 = session_with_identity().with_region("RegionOne");
        let e1 = sess1.get_catalog_endpoint("identity").unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:5000/");

        let sess2 = session_with_identity().with_region("unknown");
        match sess2.get_catalog_endpoint("identity").err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };
    }

    #[test]
    fn test_session_get_catalog_endpoint_with_region_and_endpoint_interface() {
        let session = session_with_identity().with_region("RegionOne")
            .with_endpoint_interface("admin");
        let e1 = session.get_catalog_endpoint("identity").unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:35357/");
    }
}
