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

use reqwest::{IntoUrl, Method, RequestBuilder, Url};
use reqwest::header::Headers;

use super::{ApiError, ApiResult, ApiVersion, ApiVersionRequest};
use super::auth::AuthMethod;
use super::service::{ApiVersioning, ServiceInfo, ServiceType};
use super::utils;


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
    cached_info: utils::MapCache<(&'static str, String), ServiceInfo>,
    api_versions: HashMap<&'static str, (ApiVersion, Headers)>,
    endpoint_interface: String
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
        let info = self.get_service_info_ref::<Srv>(ep)?;
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
        let key = self.ensure_service_info::<Srv>(ep)?;
        let info = self.cached_info.get_ref(&key).unwrap();

        match info.pick_api_version(requested.clone()) {
            Some(ver) => {
                let hdrs = Srv::api_version_headers(ver)?;
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
    pub fn request<U>(&self, method: Method, url: U)
            -> ApiResult<RequestBuilder> where U: IntoUrl {
        self.auth.request(method, url.into_url()?)
    }

    fn ensure_service_info<Srv>(&self, endpoint_interface: String)
            -> ApiResult<(&'static str, String)> where Srv: ServiceType {
        let key = (Srv::catalog_type(), endpoint_interface);

        self.cached_info.ensure_value(key.clone(), |_| {
            self.get_catalog_endpoint(Srv::catalog_type())
                .and_then(|ep| Srv::service_info(ep, self.auth_method()))
        })?;

        Ok(key)
    }

    fn get_catalog_endpoint<S>(&self, service_type: S) -> ApiResult<Url>
            where S: Into<String> {
        self.auth.get_endpoint(service_type.into(),
                               Some(self.endpoint_interface.clone()))
    }

    fn get_service_info_ref<Srv>(&self, endpoint_interface: String)
            -> ApiResult<Ref<ServiceInfo>> where Srv: ServiceType {
        let key = self.ensure_service_info::<Srv>(endpoint_interface)?;
        Ok(self.cached_info.get_ref(&key).unwrap())
    }
}
