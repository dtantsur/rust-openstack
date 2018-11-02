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

#![allow(dead_code)] // various things are unused with --no-default-features
#![allow(missing_docs)]

use std::collections::HashMap;
use std::str::FromStr;

use eui48::MacAddress;
use reqwest::{Method, Url};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{DeserializeOwned, Error as DeserError};
use serde_json;

use super::super::{Error, ErrorKind, Result};
use super::super::auth::AuthMethod;
use super::super::session::{RequestBuilderExt, ServiceType};
use super::super::utils;
use super::ApiVersion;

#[derive(Clone, Debug, Deserialize)]
pub struct Link {
    #[serde(deserialize_with = "deser_url")]
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
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Version {
    #[serde(deserialize_with = "deser_version")]
    pub id: ApiVersion,
    pub links: Vec<Link>,
    pub status: String,
    #[serde(deserialize_with = "empty_as_none", default)]
    pub version: Option<ApiVersion>,
    #[serde(deserialize_with = "empty_as_none", default)]
    pub min_version: Option<ApiVersion>
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Root {
    Versions { versions: Vec<Version> },
    Version { version: Version }
}

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
            major_version: self.id,
            current_version: self.version,
            minimum_version: self.min_version
        })
    }
}

impl ServiceInfo {
    /// Generic code to extract a `ServiceInfo` from a URL.
    pub fn fetch<Srv: ServiceType>(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        let service_type = Srv::catalog_type();
        debug!("Fetching {} service info from {}", service_type, endpoint);

        // Workaround for old version of Nova returning HTTP endpoints even if
        // accessed via HTTP
        let secure = endpoint.scheme() == "https";

        let result = auth.request(Method::GET, endpoint.clone())?.send_checked();
        match result {
            Ok(mut resp) => {
                let mut info = match resp.json()? {
                    Root::Version { version: ver } => {
                        trace!("The major version for {} service from {}: {:?}",
                               service_type, endpoint, ver);
                        if Srv::major_version_supported(ver.id) {
                            ver.into_service_info()
                        } else {
                            Err(Error::new(ErrorKind::EndpointNotFound,
                                           "Major version not supported"))
                        }
                    },
                    Root::Versions { versions: mut vers } => {
                        vers.sort_unstable_by_key(|x| x.id);
                        trace!("Available major versions for {} service from {}: {:?}",
                               service_type, endpoint, vers);
                        match vers.into_iter()
                                .rfind(|x| Srv::major_version_supported(x.id)) {
                            Some(ver) => ver.into_service_info(),
                            None => Err(Error::new_endpoint_not_found(service_type))
                        }
                    }
                }?;

                // Older Nova returns insecure URLs even for secure protocol.
                if secure {
                    info.root_url.set_scheme("https").unwrap();
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
                    ServiceInfo::fetch::<Srv>(utils::url::pop(endpoint, true),
                                              auth)
                }
            },
            Err(other) => Err(other)
        }
    }
}

/// Deserialize value where empty string equals None.
pub fn empty_as_none<'de, D, T>(des: D) -> ::std::result::Result<Option<T>, D::Error>
        where D: Deserializer<'de>, T: DeserializeOwned {
    let value = serde_json::Value::deserialize(des)?;
    match value {
        serde_json::Value::String(ref s) if s == "" => return Ok(None),
        _ => ()
    };

    serde_json::from_value(value).map_err(DeserError::custom)
}

/// Deserialize value where empty string equals None.
pub fn empty_as_default<'de, D, T>(des: D) -> ::std::result::Result<T, D::Error>
        where D: Deserializer<'de>, T: DeserializeOwned + Default {
    let value = serde_json::Value::deserialize(des)?;
    match value {
        serde_json::Value::String(ref s) if s == "" =>
            return Ok(Default::default()),
        _ => ()
    };

    serde_json::from_value(value).map_err(DeserError::custom)
}

pub fn deser_version<'de, D>(des: D)
        -> ::std::result::Result<ApiVersion, D::Error>
        where D: Deserializer<'de> {
    let value = String::deserialize(des)?;
    if value.is_empty() {
        return Err(D::Error::custom("Empty version ID"));
    }

    let version_part = if value.starts_with("v") {
        &value[1..]
    } else {
        &value
    };

    ApiVersion::from_str(version_part).map_err(D::Error::custom)
}

/// Deserialize a URL.
pub fn deser_url<'de, D>(des: D) -> ::std::result::Result<Url, D::Error>
        where D: Deserializer<'de> {
    Url::parse(&String::deserialize(des)?).map_err(DeserError::custom)
}

/// Deserialize a URL.
pub fn deser_optional_url<'de, D>(des: D)
        -> ::std::result::Result<Option<Url>, D::Error>
        where D: Deserializer<'de> {
    let value: Option<String> = Deserialize::deserialize(des)?;
    match value {
        Some(s) => Url::parse(&s).map_err(DeserError::custom).map(Some),
        None => Ok(None)
    }
}

/// Deserialize a key-value mapping.
pub fn deser_key_value<'de, D>(des: D)
        -> ::std::result::Result<HashMap<String, String>, D::Error>
        where D: Deserializer<'de> {
    let value: Vec<KeyValue> = Deserialize::deserialize(des)?;
    Ok(value.into_iter().map(|kv| (kv.key, kv.value)).collect())
}

/// Serialize a MAC address in its HEX format.
pub fn ser_mac<S>(value: &MacAddress, serializer: S)
        -> ::std::result::Result<S::Ok, S::Error>
        where S: Serializer {
    value.to_hex_string().serialize(serializer)
}

/// Serialize a MAC address in its HEX format.
pub fn ser_opt_mac<S>(value: &Option<MacAddress>, serializer: S)
        -> ::std::result::Result<S::Ok, S::Error>
        where S: Serializer {
    value.map(|m| m.to_hex_string()).serialize(serializer)
}
