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

use std::cell::Ref;
use std::collections::HashMap;

use hyper::{Client, Get, Url};
use hyper::client::{IntoUrl, Response};
use hyper::header::Headers;
use hyper::method::Method;
use serde::Deserialize;

use super::{ApiError, ApiResult, ApiVersion, ApiVersionRequest};
use super::auth::AuthMethod;
use super::service::{ApiVersioning, RequestBuilder, ServiceInfo, ServiceType};
use super::utils;


/// An OpenStack API session.
///
/// The session object serves as a wrapper around an HTTP(s) client, handling
/// authentication, accessing the service catalog and token refresh.
///
/// The session object also owns region and endpoint interface to use.
///
/// Finally, the session object is responsible for API version negotiation.
#[derive(Debug)]
pub struct Session {
    auth: Box<AuthMethod>,
    client: Client,
    cached_info: utils::MapCache<(&'static str, String), ServiceInfo>,
    api_versions: HashMap<&'static str, (ApiVersion, Headers)>,
    region: Option<String>,
    endpoint_interface: String
}


impl Session {
    /// Create a new session with a given authentication plugin.
    ///
    /// The resulting session will use the default endpoint interface (usually,
    /// public) and the first available region.
    pub fn new<Auth: AuthMethod + 'static>(auth_method: Auth) -> Session {
        let ep = auth_method.default_endpoint_interface();
        let region = auth_method.default_region();
        Session {
            auth: Box::new(auth_method),
            client: utils::http_client(),
            cached_info: utils::MapCache::new(),
            api_versions: HashMap::new(),
            region: region,
            endpoint_interface: ep
        }
    }

    /// Convert this session into one using the given region.
    ///
    /// Negotiated API versions are reset to their default values.
    pub fn with_region<S: Into<String>>(self, region: S) -> Session {
        Session {
            auth: self.auth,
            client: self.client,
            // ServiceInfo has to be refreshed
            cached_info: utils::MapCache::new(),
            // Different regions potentially have different API versions?
            api_versions: HashMap::new(),
            region: Some(region.into()),
            endpoint_interface: self.endpoint_interface
        }
    }

    /// Convert this session into one using the given endpoint interface.
    ///
    /// Negotiated API versions are kept in the new object.
    pub fn with_endpoint_interface<S>(self, endpoint_interface: S)
            -> Session where S: Into<String> {
        Session {
            auth: self.auth,
            client: self.client,
            // ServiceInfo has to be refreshed
            cached_info: utils::MapCache::new(),
            api_versions: self.api_versions,
            region: self.region,
            endpoint_interface: endpoint_interface.into()
        }
    }

    /// Get a reference to the authentication method in use.
    pub fn auth_method(&self) -> &AuthMethod {
        self.auth.as_ref()
    }

    /// Get an API version used for given service.
    pub fn api_version<Srv: ServiceType>(&self) -> Option<ApiVersion> {
        self.api_versions.get(Srv::catalog_type()).map(|x| x.0)
    }

    /// Get a copy of headers to send for given service.
    ///
    /// Currently only includes API version headers.
    pub fn service_headers<Srv: ServiceType>(&self) -> Headers {
        self.api_versions.get(Srv::catalog_type()).map(|x| x.1.clone())
            .unwrap_or_else(Headers::new)
    }

    /// Get service info for the given service.
    ///
    /// If endpoint interface is not provided, the default for this session
    /// is used.
    pub fn get_service_info<Srv>(&self, endpoint_interface: Option<String>)
            -> ApiResult<ServiceInfo> where Srv: ServiceType {
        let ep = endpoint_interface.unwrap_or(self.endpoint_interface.clone());
        let info = try!(self.get_service_info_ref::<Srv>(ep));
        Ok(info.clone())
    }

    /// Negotiate an API version with the service.
    ///
    /// Negotiation is based on version information returned from the root
    /// endpoint. If no minimum version is returned, the current version is
    /// assumed to be the only supported version.
    ///
    /// The resulting API version is cached for this session.
    pub fn negotiate_api_version<Srv>(&mut self, requested: ApiVersionRequest)
            -> ApiResult<ApiVersion>
            where Srv: ServiceType + ApiVersioning {
        let ep = self.endpoint_interface.clone();
        let key = try!(self.ensure_service_info::<Srv>(ep));
        let info = self.cached_info.get_ref(&key).unwrap();

        match info.pick_api_version(requested.clone()) {
            Some(ver) => {
                let hdrs = try!(Srv::api_version_headers(ver));
                let _ = self.api_versions.insert(Srv::catalog_type(),
                                                 (ver, hdrs));
                info!("Negotiated API version {} for {} API",
                      ver, Srv::catalog_type());
                Ok(ver)
            },
            None => {
                let error = ApiError::UnsupportedApiVersion {
                    requested: requested,
                    minimum: info.minimum_version.clone(),
                    maximum: info.current_version.clone()
                };
                warn!("API negotiation failed for {} API: {}",
                      Srv::catalog_type(), error);
                Err(error)
            }
        }
    }

    /// Prepare an HTTP request with authentication.
    pub fn request<'a, U>(&'a self, method: Method, url: U, headers: Headers)
            -> ApiResult<RequestBuilder<'a>> where U: IntoUrl {
        let real_url = try!(url.into_url());
        self.auth.request(&self.client, method, real_url, headers)
    }

    /// Send a simple GET request.
    pub fn http_get<U>(&self, url: U) -> ApiResult<Response> where U: IntoUrl {
        try!(self.request(Get, url, Headers::new())).send()
    }

    /// Send a simple GET request returning a JSON
    pub fn get_json<U, D>(&self, url: U) -> ApiResult<D>
            where U: IntoUrl, D: Deserialize {
        try!(self.request(Get, url, Headers::new())).fetch_json()
    }

    fn ensure_service_info<Srv>(&self, endpoint_interface: String)
            -> ApiResult<(&'static str, String)> where Srv: ServiceType {
        let key = (Srv::catalog_type(), endpoint_interface);

        try!(self.cached_info.ensure_value(key.clone(), |_| {
            self.get_catalog_endpoint(Srv::catalog_type())
                .and_then(|ep| Srv::service_info(ep, self))
        }));

        Ok(key)
    }

    fn get_catalog_endpoint<S>(&self, service_type: S) -> ApiResult<Url>
            where S: Into<String> {
        self.auth.get_endpoint(&self.client,
                               service_type.into(),
                               Some(self.endpoint_interface.clone()),
                               self.region.clone())
    }

    fn get_service_info_ref<Srv>(&self, endpoint_interface: String)
            -> ApiResult<Ref<ServiceInfo>> where Srv: ServiceType {
        let key = try!(self.ensure_service_info::<Srv>(endpoint_interface));
        Ok(self.cached_info.get_ref(&key).unwrap())
    }
}


impl Clone for Session {
    fn clone(&self) -> Session {
        Session {
            auth: self.auth.clone(),
            // NOTE: hyper::Client does not support Clone
            client: utils::http_client(),
            cached_info: self.cached_info.clone(),
            api_versions: self.api_versions.clone(),
            region: self.region.clone(),
            endpoint_interface: self.endpoint_interface.clone()
        }
    }
}


#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use std::collections::HashMap;
    use std::io::Read;

    use hyper;
    use hyper::header::{ContentLength, Headers};
    use hyper::status::StatusCode;

    use super::super::ApiError;
    use super::super::auth::{AuthMethod, Identity, NoAuth};
    use super::super::utils;
    use super::Session;

    pub fn new_with_params<Auth>(auth: Auth, cli: hyper::Client,
                                 region: Option<&str>)
            -> Session where Auth: AuthMethod + 'static {
        let ep = auth.default_endpoint_interface();
        Session {
            auth: Box::new(auth),
            client: cli,
            cached_info: utils::MapCache::new(),
            api_versions: HashMap::new(),
            region: region.map(From::from),
            endpoint_interface: ep
        }
    }

    fn session_with_identity() -> Session {
        let id = Identity::new("http://127.0.2.1").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockCatalog::default());
        let ep = id.default_endpoint_interface();
        Session {
            auth: Box::new(id),
            client: cli,
            cached_info: utils::MapCache::new(),
            api_versions: HashMap::new(),
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
    fn test_session_request() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = new_with_params(NoAuth::new("http://127.0.0.1/").unwrap(),
                                cli, None);

        let mut resp = s.request(hyper::Post, "http://127.0.0.1/",
                                 Headers::new()).unwrap()
            .body("body").header(ContentLength(4u64)).send().unwrap();

        let mut s = String::new();
        resp.read_to_string(&mut s).unwrap();
        assert_eq!(&s, "{}");
    }

    #[test]
    fn test_session_request_error() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = new_with_params(NoAuth::new("http://127.0.0.2/").unwrap(),
                                cli, None);

        let err = s.request(hyper::Post, "http://127.0.0.2/",
                            Headers::new()).unwrap()
            .body("body").header(ContentLength(4u64)).send().err().unwrap();

        match err {
            ApiError::HttpError(StatusCode::NotFound, ..) => (),
            other => panic!("Unexpected {}", other)
        }
    }

    #[test]
    fn test_session_request_unchecked_error() {
        let cli = hyper::Client::with_connector(MockHttp::default());
        let s = new_with_params(NoAuth::new("http://127.0.0.2/").unwrap(),
                                cli, None);

        let mut resp = s.request(hyper::Post, "http://127.0.0.2/",
                                 Headers::new()).unwrap()
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
