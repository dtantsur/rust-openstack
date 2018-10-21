// Copyright 2018 Dmitry Tantsur <divius.inside@gmail.com>
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

use log;
use reqwest::{Body, Method, RequestBuilder as ReqwestRB, Response, Url};
use reqwest::header::{Header, Headers};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::Result;
use super::auth::AuthMethod;
use super::common::ApiVersion;
use super::utils;

/// Information about API endpoint.
#[derive(Clone, Debug)]
pub struct ServiceInfo {
    /// Root endpoint.
    pub root_url: Url,
    /// Major API version.
    pub major_version: ApiVersion,
    /// Current API version (if supported).
    pub current_version: Option<ApiVersion>,
    /// Minimum API version (if supported).
    pub minimum_version: Option<ApiVersion>
}

/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Get basic service information.
    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo>;

    /// Return headers to set for this API version.
    fn api_version_headers(_version: ApiVersion) -> Option<Headers> { None }
}

/// An HTTP request builder.
///
/// This is a thin wrapper around reqwest's RequestBuilder with error handling.
#[derive(Debug)]
pub struct RequestBuilder {
    inner: ReqwestRB,
}

impl RequestBuilder {
    /// Create a RequestBuilder by wrapping a reqwest's one.
    pub fn new(inner: ReqwestRB) -> RequestBuilder {
        RequestBuilder {
            inner: inner
        }
    }

    /// Access to the inner object.
    pub fn inner_mut(&mut self) -> &mut ReqwestRB {
        &mut self.inner
    }

    /// Take the inner object out.
    pub fn into_inner(self) -> ReqwestRB {
        self.inner
    }

    /// Add a Header to this Request.
    pub fn header<H: Header>(&mut self, header: H) -> &mut RequestBuilder {
        let _ = self.inner.header(header);
        self
    }

    /// Add a set of Headers to the existing ones on this Request.
    pub fn headers(&mut self, headers: Headers) -> &mut RequestBuilder {
        let _ = self.inner.headers(headers);
        self
    }

    /// Set the request body.
    pub fn body<T: Into<Body>>(&mut self, body: T) -> &mut RequestBuilder {
        let _ = self.inner.body(body);
        self
    }

    /// Modify the query string of the URL.
    pub fn query<T: Serialize>(&mut self, query: &T) -> &mut RequestBuilder {
        let _ = self.inner.query(query);
        self
    }

    /// Send a JSON body.
    pub fn json<T: Serialize>(&mut self, json: &T) -> &mut RequestBuilder {
        let _ = self.inner.json(json);
        self
    }

    /// Construct the Request and sends it the target URL, returning a Response.
    pub fn send(&mut self) -> Result<Response> {
        _log(self.inner.send()?).error_for_status().map_err(From::from)
    }

    /// Construct the Request, send it and receive a JSON.
    pub fn receive_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        _log(self.inner.send()?).error_for_status()?.json().map_err(From::from)
    }
}

fn _log(mut resp: Response) -> Response {
    if log_enabled!(log::Level::Trace) {
        let details = if resp.status().is_client_error() || resp.status().is_server_error() {
            resp.text().ok()
        } else {
            None
        };

        // TODO(dtantsur): proper error parsing
        trace!("HTTP request to {} returned {}; error: {:?}",
               resp.url(), resp.status(), details);
    }
    resp
}


/// An OpenStack API session.
///
/// The session object serves as a wrapper around an HTTP(s) client, handling
/// authentication, accessing the service catalog and token refresh.
///
/// The session object also owns the endpoint interface to use.
#[derive(Debug, Clone)]
pub struct Session {
    auth: Box<AuthMethod>,
    cached_info: utils::MapCache<&'static str, ServiceInfo>,
    endpoint_interface: String
}


impl Session {
    /// Create a new session with a given authentication plugin.
    ///
    /// The resulting session will use the default endpoint interface (usually,
    /// public).
    pub fn new<Auth: AuthMethod + 'static>(auth_method: Auth) -> Session {
        let ep = auth_method.default_endpoint_interface();
        Session {
            auth: Box::new(auth_method),
            cached_info: utils::MapCache::new(),
            endpoint_interface: ep
        }
    }

    /// Set endpoint interface to use.
    ///
    /// This call clears the cached service information.
    pub fn set_endpoint_interface<S>(&mut self, endpoint_interface: S)
            where S: Into<String> {
        self.cached_info = utils::MapCache::new();
        self.endpoint_interface = endpoint_interface.into();
    }

    /// Convert this session into one using the given endpoint interface.
    pub fn with_endpoint_interface<S>(mut self, endpoint_interface: S)
            -> Session where S: Into<String> {
        self.set_endpoint_interface(endpoint_interface);
        self
    }

    /// Get a reference to the authentication method in use.
    pub fn auth_method(&self) -> &AuthMethod {
        self.auth.as_ref()
    }

    /// Get a mutable reference to the authentication method in use.
    pub fn auth_method_mut(&mut self) -> &mut AuthMethod {
        self.auth.as_mut()
    }

    /// Get service info for the given service.
    pub fn get_service_info<Srv>(&self) -> Result<ServiceInfo>
            where Srv: ServiceType {
        let info = self.get_service_info_ref::<Srv>()?;
        Ok(info.clone())
    }

    /// Construct and endpoint for the given service from the path.
    pub fn get_endpoint<Srv: ServiceType>(&self, path: &[&str])
            -> Result<Url> {
        let info = self.get_service_info_ref::<Srv>()?;
        Ok(utils::url::extend(info.root_url.clone(), path))
    }

    /// Make an HTTP request to the given service.
    pub fn request<Srv: ServiceType>(&self, method: Method, path: &[&str],
                                     api_version: Option<ApiVersion>)
            -> Result<RequestBuilder> {
        let url = self.get_endpoint::<Srv>(path)?;
        trace!("Sending HTTP {} request to {} with API version {:?}",
               method, url, api_version);
        let maybe_headers = api_version.and_then(|ver| {
            Srv::api_version_headers(ver)
        });
        let mut builder = self.auth.request(method, url)?;
        if let Some(headers) = maybe_headers {
            let _unused = builder.headers(headers);
        }
        Ok(builder)
    }

    fn ensure_service_info<Srv>(&self) -> Result<()> where Srv: ServiceType {
        self.cached_info.ensure_value(Srv::catalog_type(), |_| {
            self.get_catalog_endpoint(Srv::catalog_type())
                .and_then(|ep| Srv::service_info(ep, self.auth_method()))
        })?;

        Ok(())
    }

    fn get_catalog_endpoint<S>(&self, service_type: S) -> Result<Url>
            where S: Into<String> {
        self.auth.get_endpoint(service_type.into(),
                               Some(self.endpoint_interface.clone()))
    }

    pub(crate) fn get_service_info_ref<Srv>(&self)
            -> Result<Ref<ServiceInfo>> where Srv: ServiceType {
        self.ensure_service_info::<Srv>()?;
        Ok(self.cached_info.get_ref(&Srv::catalog_type()).unwrap())
    }
}

impl ServiceInfo {
    /// Whether this service supports the given API version.
    ///
    /// Defaults to false if cannot be determined.
    pub fn supports_api_version(&self, version: ApiVersion) -> bool {
        match (self.minimum_version, self.current_version) {
            (Some(min), Some(max)) => min <= version && max >= version,
            (None, Some(current)) => current == version,
            (Some(min), None) => version >= min,
            _ => false
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::utils;

    #[test]
    fn test_session_new() {
        let s = utils::test::new_session(utils::test::URL);
        let ep = s.get_catalog_endpoint("fake").unwrap();
        assert_eq!(&ep.to_string(), utils::test::URL);
    }

    #[test]
    fn test_session_get_endpoint() {
        let s = utils::test::new_session(utils::test::URL);
        let ep = s.get_endpoint::<utils::test::FakeServiceType>(&[])
            .unwrap();
        assert_eq!(&ep.to_string(), utils::test::URL);
    }

    #[test]
    fn test_session_get_endpoint_with_path() {
        let s = utils::test::new_session(utils::test::URL);
        let ep = s.get_endpoint::<utils::test::FakeServiceType>(&["foo", "bar"])
            .unwrap();
        assert_eq!(ep.to_string(), format!("{}foo/bar", utils::test::URL));
    }
}
