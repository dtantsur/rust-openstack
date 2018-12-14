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
use reqwest::{Method, RequestBuilder, Response, Url};
use serde::de::DeserializeOwned;

use super::{Error, ErrorKind, Result};
use super::auth::AuthMethod;
use super::common::ApiVersion;
use super::common::protocol::ServiceInfo;
use super::utils;


/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Check whether this service type is compatible with the given major version.
    fn major_version_supported(_version: ApiVersion) -> bool { true }

    /// Update the request to include the API version headers.
    ///
    /// The default implementation fails with `IncompatibleApiVersion`.
    fn set_api_version_headers(_request: RequestBuilder, _version: ApiVersion)
            -> Result<RequestBuilder> {
        Err(Error::new(ErrorKind::IncompatibleApiVersion,
                       format!("The {} service does not support API versions",
                               Self::catalog_type())))
    }

    /// Whether this service supports version discovery at all.
    fn version_discovery_supported() -> bool { true }
}

/// Extension trait for HTTP calls with error handling.
pub trait RequestBuilderExt {
    /// Send a request and validate the status code.
    fn send_checked(self) -> Result<Response>;

    /// Send a request and discard the results.
    fn commit(self) -> Result<()> where Self: Sized {
        let _ = self.send_checked()?;
        Ok(())
    }

    /// Send a request and receive a JSON back.
    fn receive_json<T: DeserializeOwned>(self) -> Result<T> where Self: Sized {
        self.send_checked()?.json().map_err(From::from)
    }
}

impl RequestBuilderExt for RequestBuilder {
    fn send_checked(self) -> Result<Response> {
        _log(self.send()?).error_for_status().map_err(From::from)
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

    /// Construct and endpoint for the given service from the path.
    pub fn get_endpoint<Srv: ServiceType>(&self, path: &[&str])
            -> Result<Url> {
        let info = self.get_service_info_ref::<Srv>()?;
        Ok(utils::url::extend(info.root_url.clone(), path))
    }

    /// Get the currently used major version from the given service.
    ///
    /// Can return `IncompatibleApiVersion` if the service does not support
    /// API version discovery at all.
    pub fn get_major_version<Srv: ServiceType>(&self) -> Result<ApiVersion> {
        let info = self.get_service_info_ref::<Srv>()?;
        info.major_version.ok_or_else(|| {
            Error::new(ErrorKind::IncompatibleApiVersion,
                       format!("{} service does not expose major version",
                               Srv::catalog_type()))
        })
    }

    /// Get minimum/maximum API (micro)version information.
    ///
    /// Returns `None` if the range cannot be determined, which usually means
    /// that microversioning is not supported.
    pub fn get_api_versions<Srv: ServiceType>(&self)
            -> Result<Option<(ApiVersion, ApiVersion)>> {
        let info = self.get_service_info_ref::<Srv>()?;
        match (info.minimum_version, info.current_version) {
            (Some(min), Some(max)) => Ok(Some((min, max))),
            _ => Ok(None)
        }
    }

    /// Make an HTTP request to the given service.
    pub fn request<Srv: ServiceType>(&self, method: Method, path: &[&str],
                                     api_version: Option<ApiVersion>)
            -> Result<RequestBuilder> {
        let url = self.get_endpoint::<Srv>(path)?;
        trace!("Sending HTTP {} request to {} with API version {:?}",
               method, url, api_version);
        let mut builder = self.auth.request(method, url)?;
        if let Some(version) = api_version {
            builder = Srv::set_api_version_headers(builder, version)?;
        }
        Ok(builder)
    }

    /// Start a GET request.
    pub fn get<Srv: ServiceType>(&self, path: &[&str], api_version: Option<ApiVersion>)
            -> Result<RequestBuilder> {
        self.request::<Srv>(Method::GET, path, api_version)
    }

    /// Start a POST request.
    pub fn post<Srv: ServiceType>(&self, path: &[&str], api_version: Option<ApiVersion>)
            -> Result<RequestBuilder> {
        self.request::<Srv>(Method::POST, path, api_version)
    }

    /// Start a PUT request.
    pub fn put<Srv: ServiceType>(&self, path: &[&str], api_version: Option<ApiVersion>)
            -> Result<RequestBuilder> {
        self.request::<Srv>(Method::PUT, path, api_version)
    }

    /// Start a DELETE request.
    pub fn delete<Srv: ServiceType>(&self, path: &[&str], api_version: Option<ApiVersion>)
            -> Result<RequestBuilder> {
        self.request::<Srv>(Method::DELETE, path, api_version)
    }

    fn ensure_service_info<Srv>(&self) -> Result<()> where Srv: ServiceType {
        self.cached_info.ensure_value(Srv::catalog_type(), |_| {
            self.get_catalog_endpoint(Srv::catalog_type())
                .and_then(|ep| ServiceInfo::fetch::<Srv>(ep, self.auth_method()))
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
    #[allow(dead_code)]  // unused with --no-default-features
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
}
