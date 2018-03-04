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

//! Common protocol bits.

#![allow(missing_docs)]

use reqwest::{Method, Url};
use serde_json;

use super::super::{Error, ErrorKind, Result};
use super::super::auth::AuthMethod;
use super::super::session::ServiceInfo;
use super::super::utils;
use super::ApiVersion;

#[derive(Clone, Debug, Deserialize)]
pub struct Link {
    #[serde(deserialize_with = "utils::deser_url")]
    pub href: Url,
    pub rel: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ref {
    pub id: String,
    pub links: Vec<Link>
}

#[derive(Clone, Debug, Deserialize)]
pub struct IdAndName {
    pub id: String,
    pub name: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub links: Vec<Link>,
    pub status: String,
    #[serde(deserialize_with = "utils::empty_as_none", default)]
    pub version: Option<ApiVersion>,
    #[serde(deserialize_with = "utils::empty_as_none", default)]
    pub min_version: Option<ApiVersion>
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionsRoot {
    pub versions: Vec<Version>
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionRoot {
    pub version: Version
}

impl Version {
    pub fn into_service_info(self) -> Result<ServiceInfo> {
        let endpoint = match self.links.into_iter().find(|x| &x.rel == "self") {
            Some(link) => link.href,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidResponse,
                    "Invalid version - missing self link"));
            }
        };

        Ok(ServiceInfo {
            root_url: endpoint,
            current_version: self.version,
            minimum_version: self.min_version
        })
    }
}

/// Generic code to extract a `ServiceInfo` from a URL.
#[allow(dead_code)] // unused with --no-default-features
pub fn fetch_service_info(endpoint: Url, auth: &AuthMethod,
                          service_type: &str, major_version: &str)
        -> Result<ServiceInfo> {
    debug!("Fetching {} service info from {}", service_type, endpoint);

    // Workaround for old version of Nova returning HTTP endpoints even if
    // accessed via HTTP
    let secure = endpoint.scheme() == "https";

    let result = auth.request(Method::Get, endpoint.clone())?.send();
    match result {
        Ok(mut resp) => {
            let body = resp.text()?;

            // First, assume it's a versioned URL.
            let mut info = match serde_json::from_str::<VersionRoot>(&body) {
                Ok(ver) => ver.version.into_service_info(),
                Err(..) => {
                    // Second, assume it's a root URL.
                    let vers = serde_json::from_str::<VersionsRoot>(&body)
                        .map_err(|e| {
                            let msg = format!("Malformed version root of the {} service: {}",
                                              service_type, e);
                            Error::new(ErrorKind::InvalidResponse, msg)
                        })?;
                    match vers.versions.into_iter().find(|x| &x.id == major_version) {
                        Some(ver) => ver.into_service_info(),
                        None => Err(Error::new_endpoint_not_found(service_type))
                    }
                }
            }?;

            // Older Nova returns insecure URLs even for secure protocol.
            if secure {
                let _ = info.root_url.set_scheme("https").unwrap();
            }

            debug!("Received {:?} for {} service from {}",
                   info, service_type, endpoint);
            Ok(info)
        },
        Err(ref e) if e.kind() == ErrorKind::ResourceNotFound => {
            if utils::url::is_root(&endpoint) {
                Err(Error::new_endpoint_not_found(service_type))
            } else {
                debug!("Got HTTP 404 from {}, trying parent endpoint",
                       endpoint);
                fetch_service_info(utils::url::pop(endpoint, true), auth,
                                   service_type, major_version)
            }
        },
        Err(other) => Err(other)
    }
}
