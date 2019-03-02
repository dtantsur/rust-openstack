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
use serde::de::{DeserializeOwned, Error as DeserError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json;

use super::super::auth::AuthMethod;
use super::super::session::{RequestBuilderExt, ServiceType};
use super::super::utils;
use super::super::{Error, ErrorKind, Result};
use super::ApiVersion;

#[derive(Clone, Debug, Deserialize)]
pub struct Link {
    #[serde(deserialize_with = "deser_url")]
    pub href: Url,
    pub rel: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ref {
    pub id: String,
    pub links: Vec<Link>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IdAndName {
    pub id: String,
    pub name: String,
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
    #[serde(deserialize_with = "empty_as_none", default)]
    pub status: Option<String>,
    #[serde(deserialize_with = "empty_as_none", default)]
    pub version: Option<ApiVersion>,
    #[serde(deserialize_with = "empty_as_none", default)]
    pub min_version: Option<ApiVersion>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Root {
    MultipleVersions { versions: Vec<Version> },
    OneVersion { version: Version },
}

/// Information about API endpoint.
#[derive(Clone, Debug)]
pub struct ServiceInfo {
    /// Root endpoint.
    pub root_url: Url,
    /// Major API version.
    pub major_version: Option<ApiVersion>,
    /// Current API version (if supported).
    pub current_version: Option<ApiVersion>,
    /// Minimum API version (if supported).
    pub minimum_version: Option<ApiVersion>,
}

impl Version {
    pub fn is_stable(&self) -> bool {
        if let Some(ref status) = self.status {
            let upper = status.to_uppercase();
            upper == "STABLE" || upper == "CURRENT" || upper == "SUPPORTED"
        } else {
            true
        }
    }

    pub fn into_service_info(self) -> Result<ServiceInfo> {
        let endpoint = match self.links.into_iter().find(|x| &x.rel == "self") {
            Some(link) => link.href,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidResponse,
                    "Invalid version - missing self link",
                ));
            }
        };

        Ok(ServiceInfo {
            root_url: endpoint,
            major_version: Some(self.id),
            current_version: self.version,
            minimum_version: self.min_version,
        })
    }
}

impl Root {
    /// Fetch versioning root from a URL.
    pub fn fetch<Srv: ServiceType>(endpoint: Url, auth: &AuthMethod) -> Result<Root> {
        debug!(
            "Fetching {} service info from {}",
            Srv::catalog_type(),
            endpoint
        );

        let result = auth.request(Method::GET, endpoint.clone())?.send_checked();
        match result {
            Ok(mut resp) => resp.json::<Root>().map_err(From::from),
            Err(ref e) if e.kind() == ErrorKind::ResourceNotFound => {
                if utils::url::is_root(&endpoint) {
                    Err(utils::endpoint_not_found(Srv::catalog_type()))
                } else {
                    debug!("Got HTTP 404 from {}, trying parent endpoint", endpoint);
                    Root::fetch::<Srv>(utils::url::pop(endpoint, true), auth)
                }
            }
            Err(other) => Err(other),
        }
    }

    /// Extract `ServiceInfo` from a version discovery root.
    pub fn into_service_info<Srv: ServiceType>(self) -> Result<ServiceInfo> {
        trace!(
            "Available major versions for {} service: {:?}",
            Srv::catalog_type(),
            self
        );

        match self {
            Root::OneVersion { version: ver } => {
                if Srv::major_version_supported(ver.id) {
                    if !ver.is_stable() {
                        warn!(
                            "Using version {:?} of {} API that is not marked as stable",
                            ver,
                            Srv::catalog_type()
                        );
                    }

                    ver.into_service_info()
                } else {
                    Err(Error::new(
                        ErrorKind::EndpointNotFound,
                        "Major version not supported",
                    ))
                }
            }
            Root::MultipleVersions { versions: mut vers } => {
                vers.sort_unstable_by_key(|x| x.id);
                match vers
                    .into_iter()
                    .rfind(|x| x.is_stable() && Srv::major_version_supported(x.id))
                {
                    Some(ver) => ver.into_service_info(),
                    None => Err(utils::endpoint_not_found(Srv::catalog_type())),
                }
            }
        }
    }
}

impl ServiceInfo {
    /// Generic code to extract a `ServiceInfo` from a URL.
    pub fn fetch<Srv: ServiceType>(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        if !Srv::version_discovery_supported() {
            debug!(
                "Service {} does not support version discovery, using {}",
                Srv::catalog_type(),
                endpoint
            );
            return Ok(ServiceInfo {
                root_url: endpoint,
                major_version: None,
                current_version: None,
                minimum_version: None,
            });
        }

        // Workaround for old version of Nova returning HTTP endpoints even if
        // accessed via HTTP
        let secure = endpoint.scheme() == "https";

        let root = Root::fetch::<Srv>(endpoint, auth)?;
        let mut info = root.into_service_info::<Srv>()?;

        // Older Nova returns insecure URLs even for secure protocol.
        if secure && info.root_url.scheme() == "http" {
            info.root_url.set_scheme("https").unwrap();
        }

        debug!("Received {:?} for {} service", info, Srv::catalog_type());
        Ok(info)
    }
}

/// Deserialize value where empty string equals None.
pub fn empty_as_none<'de, D, T>(des: D) -> ::std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(des)?;
    match value {
        serde_json::Value::String(ref s) if s == "" => return Ok(None),
        _ => (),
    };

    serde_json::from_value(value).map_err(DeserError::custom)
}

/// Deserialize value where empty string equals None.
pub fn empty_as_default<'de, D, T>(des: D) -> ::std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = serde_json::Value::deserialize(des)?;
    match value {
        serde_json::Value::String(ref s) if s == "" => return Ok(Default::default()),
        _ => (),
    };

    serde_json::from_value(value).map_err(DeserError::custom)
}

pub fn deser_version<'de, D>(des: D) -> ::std::result::Result<ApiVersion, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(des)?;
    if value.is_empty() {
        return Err(D::Error::custom("Empty version ID"));
    }

    let version_part = if value.starts_with('v') {
        &value[1..]
    } else {
        &value
    };

    ApiVersion::from_str(version_part).map_err(D::Error::custom)
}

/// Deserialize a URL.
pub fn deser_url<'de, D>(des: D) -> ::std::result::Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    Url::parse(&String::deserialize(des)?).map_err(DeserError::custom)
}

/// Deserialize a URL.
pub fn deser_optional_url<'de, D>(des: D) -> ::std::result::Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(des)?;
    match value {
        Some(s) => Url::parse(&s).map_err(DeserError::custom).map(Some),
        None => Ok(None),
    }
}

/// Deserialize a key-value mapping.
pub fn deser_key_value<'de, D>(des: D) -> ::std::result::Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Vec<KeyValue> = Deserialize::deserialize(des)?;
    Ok(value.into_iter().map(|kv| (kv.key, kv.value)).collect())
}

/// Serialize a MAC address in its HEX format.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn ser_mac<S>(value: &MacAddress, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.to_hex_string().serialize(serializer)
}

/// Serialize a MAC address in its HEX format.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn ser_opt_mac<S>(
    value: &Option<MacAddress>,
    serializer: S,
) -> ::std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.map(|m| m.to_hex_string()).serialize(serializer)
}

#[cfg(test)]
mod test {
    use reqwest::Url;

    use super::super::super::session::ServiceType;
    use super::super::super::ErrorKind;
    use super::super::ApiVersion;
    use super::{Link, Root, Version};

    #[test]
    fn test_version_current_is_stable() {
        let stable = Version {
            id: ApiVersion(2, 0),
            links: Vec::new(),
            status: Some("CURRENT".to_string()),
            version: None,
            min_version: None,
        };
        assert!(stable.is_stable());
    }

    #[test]
    fn test_version_stable_is_stable() {
        let stable = Version {
            id: ApiVersion(2, 0),
            links: Vec::new(),
            status: Some("Stable".to_string()),
            version: None,
            min_version: None,
        };
        assert!(stable.is_stable());
    }

    #[test]
    fn test_version_supported_is_stable() {
        let stable = Version {
            id: ApiVersion(2, 0),
            links: Vec::new(),
            status: Some("supported".to_string()),
            version: None,
            min_version: None,
        };
        assert!(stable.is_stable());
    }

    #[test]
    fn test_version_no_status_is_stable() {
        let stable = Version {
            id: ApiVersion(2, 0),
            links: Vec::new(),
            status: None,
            version: None,
            min_version: None,
        };
        assert!(stable.is_stable());
    }

    #[test]
    fn test_version_deprecated_is_not_stable() {
        let unstable = Version {
            id: ApiVersion(2, 0),
            links: Vec::new(),
            status: Some("DEPRECATED".to_string()),
            version: None,
            min_version: None,
        };
        assert!(!unstable.is_stable());
    }

    #[test]
    fn test_version_into_service_info() {
        let url = Url::parse("https://example.com/v2").unwrap();
        let ver = Version {
            id: ApiVersion(2, 0),
            links: vec![
                Link {
                    href: Url::parse("https://example.com/docs").unwrap(),
                    rel: "other".to_string(),
                },
                Link {
                    href: url.clone(),
                    rel: "self".to_string(),
                },
            ],
            status: None,
            version: Some(ApiVersion(2, 2)),
            min_version: None,
        };
        let info = ver.into_service_info().unwrap();
        assert_eq!(info.root_url, url);
        assert_eq!(info.major_version, Some(ApiVersion(2, 0)));
        assert_eq!(info.current_version, Some(ApiVersion(2, 2)));
        assert_eq!(info.minimum_version, None);
    }

    #[test]
    fn test_version_into_service_info_no_self_link() {
        let ver = Version {
            id: ApiVersion(2, 0),
            links: vec![Link {
                href: Url::parse("https://example.com/docs").unwrap(),
                rel: "other".to_string(),
            }],
            status: None,
            version: Some(ApiVersion(2, 2)),
            min_version: None,
        };
        let err = ver.into_service_info().err().unwrap();
        assert_eq!(err.kind(), ErrorKind::InvalidResponse);
    }

    struct ServiceWithDiscovery;

    impl ServiceType for ServiceWithDiscovery {
        fn catalog_type() -> &'static str {
            "test-service-with-discovery"
        }

        fn major_version_supported(version: ApiVersion) -> bool {
            version.0 == 1 && version.1 > 0
        }
    }

    #[test]
    fn test_root_into_service_info_one_version() {
        let url = Url::parse("https://example.com/v1.2").unwrap();
        let root = Root::OneVersion {
            version: Version {
                id: ApiVersion(1, 2),
                links: vec![Link {
                    href: url.clone(),
                    rel: "self".to_string(),
                }],
                status: Some("STABLE".to_string()),
                version: None,
                min_version: None,
            },
        };

        let info = root.into_service_info::<ServiceWithDiscovery>().unwrap();
        assert_eq!(info.root_url, url);
        assert_eq!(info.major_version, Some(ApiVersion(1, 2)));
    }

    #[test]
    fn test_root_into_service_info_one_version_unsupported() {
        let url = Url::parse("https://example.com/v1.0").unwrap();
        let root = Root::OneVersion {
            version: Version {
                id: ApiVersion(1, 0),
                links: vec![Link {
                    href: url.clone(),
                    rel: "self".to_string(),
                }],
                status: Some("STABLE".to_string()),
                version: None,
                min_version: None,
            },
        };

        let err = root
            .into_service_info::<ServiceWithDiscovery>()
            .err()
            .unwrap();
        assert_eq!(err.kind(), ErrorKind::EndpointNotFound);
    }

    #[test]
    fn test_root_into_service_info_versions() {
        let url = Url::parse("https://example.com/v1.2").unwrap();
        let root = Root::MultipleVersions {
            versions: vec![
                Version {
                    id: ApiVersion(1, 0),
                    links: vec![Link {
                        href: Url::parse("https://example.com/1.0").unwrap(),
                        rel: "self".to_string(),
                    }],
                    status: Some("STABLE".to_string()),
                    version: None,
                    min_version: None,
                },
                Version {
                    id: ApiVersion(1, 1),
                    links: vec![Link {
                        href: Url::parse("https://example.com/1.1").unwrap(),
                        rel: "self".to_string(),
                    }],
                    status: Some("STABLE".to_string()),
                    version: None,
                    min_version: None,
                },
                Version {
                    id: ApiVersion(1, 2),
                    links: vec![Link {
                        href: url.clone(),
                        rel: "self".to_string(),
                    }],
                    status: Some("STABLE".to_string()),
                    version: None,
                    min_version: None,
                },
                Version {
                    id: ApiVersion(2, 0),
                    links: vec![Link {
                        href: Url::parse("https://example.com/2.0").unwrap(),
                        rel: "self".to_string(),
                    }],
                    status: Some("STABLE".to_string()),
                    version: None,
                    min_version: None,
                },
            ],
        };

        let info = root.into_service_info::<ServiceWithDiscovery>().unwrap();
        assert_eq!(info.root_url, url);
        assert_eq!(info.major_version, Some(ApiVersion(1, 2)));
    }

    #[test]
    fn test_root_into_service_info_versions_unsupported() {
        let root = Root::MultipleVersions {
            versions: vec![
                Version {
                    id: ApiVersion(1, 0),
                    links: vec![Link {
                        href: Url::parse("https://example.com/1.0").unwrap(),
                        rel: "self".to_string(),
                    }],
                    status: Some("STABLE".to_string()),
                    version: None,
                    min_version: None,
                },
                Version {
                    id: ApiVersion(2, 0),
                    links: vec![Link {
                        href: Url::parse("https://example.com/2.0").unwrap(),
                        rel: "self".to_string(),
                    }],
                    status: Some("STABLE".to_string()),
                    version: None,
                    min_version: None,
                },
            ],
        };

        let err = root
            .into_service_info::<ServiceWithDiscovery>()
            .err()
            .unwrap();
        assert_eq!(err.kind(), ErrorKind::EndpointNotFound);
    }
}
