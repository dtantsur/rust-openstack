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

use reqwest::{Body, Method, RequestBuilder as ReqwestRB, Response, Url};
use reqwest::header::{Header, Headers};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{Error, ErrorKind, Result, ApiVersion};
use super::auth::AuthMethod;
use super::service::{ApiVersioning, ServiceInfo, ServiceType};
use super::utils;

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

#[inline]
fn _log(resp: Response) -> Response {
    trace!("HTTP request to {} returned {}", resp.url(), resp.status());
    resp
}


/// An OpenStack API session.
///
/// The session object serves as a wrapper around an HTTP(s) client, handling
/// authentication, accessing the service catalog and token refresh.
///
/// The session object also owns region and endpoint interface to use.
///
/// Finally, the session object is responsible for API version negotiation.
#[derive(Debug, Clone)]
pub struct Session {
    auth: Box<AuthMethod>,
    cached_info: utils::MapCache<&'static str, ServiceInfo>,
    api_versions: HashMap<&'static str, ApiVersion>,
    endpoint_interface: String
}

/// A request for negotiating an API version.
#[derive(Debug, Clone)]
pub enum ApiVersionRequest {
    /// Minimum possible version (usually the default).
    Minimum,
    /// Latest version.
    ///
    /// This may result in an incompatible version, so it is always recommended
    /// to use LatestFrom or Choice instead.
    Latest,
    /// Latest version from the given range.
    LatestFrom(ApiVersion, ApiVersion),
    /// Specified version.
    ///
    /// This is a very inflexible approach, and is only recommended when the
    /// application can work with one and only one version.
    Exact(ApiVersion),
    /// Choice between several versions.
    Choice(Vec<ApiVersion>)
}


impl Session {
    /// Create a new session with a given authentication plugin.
    ///
    /// The resulting session will use the default endpoint interface (usually,
    /// public) and the first available region.
    pub fn new<Auth: AuthMethod + 'static>(auth_method: Auth) -> Session {
        let ep = auth_method.default_endpoint_interface();
        Session {
            auth: Box::new(auth_method),
            cached_info: utils::MapCache::new(),
            api_versions: HashMap::new(),
            endpoint_interface: ep
        }
    }

    /// Convert this session into one using the given endpoint interface.
    ///
    /// Negotiated API versions are kept in the new object.
    pub fn with_endpoint_interface<S>(self, endpoint_interface: S)
            -> Session where S: Into<String> {
        Session {
            auth: self.auth,
            // ServiceInfo has to be refreshed
            cached_info: utils::MapCache::new(),
            api_versions: self.api_versions,
            endpoint_interface: endpoint_interface.into()
        }
    }

    /// Get a reference to the authentication method in use.
    pub fn auth_method(&self) -> &AuthMethod {
        self.auth.as_ref()
    }

    /// Get a mutable reference to the authentication method in use.
    pub fn auth_method_mut(&mut self) -> &mut AuthMethod {
        self.auth.as_mut()
    }

    /// Get an API version used for given service.
    pub fn api_version<Srv: ServiceType>(&self) -> Option<ApiVersion> {
        self.api_versions.get(Srv::catalog_type()).cloned()
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
    pub fn request<Srv: ServiceType>(&self, method: Method, path: &[&str])
            -> Result<RequestBuilder> {
        let url = self.get_endpoint::<Srv>(path)?;
        let maybe_headers = self.api_versions.get(Srv::catalog_type())
            .and_then(|ver| Srv::api_version_headers(*ver));
        trace!("Sending HTTP {} request to {} with headers {:?}",
               method, url, maybe_headers);
        let mut builder = self.auth.request(method, url)?;
        if let Some(headers) = maybe_headers {
            let _unused = builder.headers(headers);
        }
        Ok(builder)
    }

    /// Negotiate an API version with the service.
    ///
    /// Negotiation is based on version information returned from the root
    /// endpoint. If no minimum version is returned, the current version is
    /// assumed to be the only supported version.
    ///
    /// The resulting API version is cached for this session.
    pub fn negotiate_api_version<Srv>(&mut self, requested: ApiVersionRequest)
            -> Result<ApiVersion>
            where Srv: ServiceType + ApiVersioning {
        self.ensure_service_info::<Srv>()?;
        let info = self.cached_info.get_ref(&Srv::catalog_type()).unwrap();

        match info.pick_api_version(requested.clone()) {
            Some(ver) => {
                let _ = self.api_versions.insert(Srv::catalog_type(), ver);
                info!("Negotiated API version {} for {} API",
                      ver, Srv::catalog_type());
                Ok(ver)
            },
            None => {
                let msg = format!(
                    "API negotiation failed for {} API: requested {:?} supported range is {:?} to {:?}",
                    Srv::catalog_type(), requested, info.minimum_version, info.current_version);
                warn!("Unable to pick API version: {}", msg);
                Err(Error::new(ErrorKind::IncompatibleApiVersion, msg))
            }
        }
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

    fn get_service_info_ref<Srv>(&self)
            -> Result<Ref<ServiceInfo>> where Srv: ServiceType {
        self.ensure_service_info::<Srv>()?;
        Ok(self.cached_info.get_ref(&Srv::catalog_type()).unwrap())
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
