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

//! Common code.

use std::fmt;
use std::str::FromStr;

use reqwest::{Method, Response, StatusCode, Url, UrlError};
use reqwest::Error as HttpClientError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error as DeserError, Visitor};
use serde_json;

use super::auth::AuthMethod;
use super::service::ServiceInfo;
use super::utils;


/// Error from an OpenStack call.
#[derive(Debug)]
pub enum Error {
    /// Requested service endpoint was not found.
    ///
    /// Contains the failed endpoint name.
    EndpointNotFound(String),

    /// Invalid value passed to one of paremeters.
    ///
    /// Contains the error message.
    InvalidInput(String),

    /// Invalid URL.
    InvalidUrl(UrlError),

    /// Generic HTTP error.
    HttpError(StatusCode, Response),

    /// Protocol-level error reported by underlying HTTP library.
    ProtocolError(HttpClientError),

    /// Response received from the server is malformed.
    ///
    /// Contains the error message.
    InvalidResponse(String),

    /// Malformed API version.
    #[allow(missing_docs)]
    InvalidApiVersion { value: String, message: String },

    /// Unsupported API version.
    #[allow(missing_docs)]
    UnsupportedApiVersion {
        requested: ApiVersionRequest,
        minimum: Option<ApiVersion>,
        maximum: Option<ApiVersion>
    }
}

/// Result of an OpenStack call.
pub type Result<T> = ::std::result::Result<T, Error>;

/// API version (major, minor).
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct ApiVersion(pub u16, pub u16);

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

/// Sorting request.
#[derive(Debug, Clone)]
pub enum Sort<T: Into<String>> {
    /// Sorting by given field in ascendant order.
    Asc(T),
    /// Sorting by given field in descendant order.
    Desc(T)
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::EndpointNotFound(ref endp) =>
                write!(f, "Requested endpoint {} was not found", endp),
            Error::InvalidInput(ref msg) =>
                write!(f, "Input value(s) are invalid: {}", msg),
            Error::InvalidUrl(ref e) => fmt::Display::fmt(e, f),
            Error::HttpError(status, ..) =>
                write!(f, "HTTP error {}", status),
            Error::ProtocolError(ref e) => fmt::Display::fmt(e, f),
            Error::InvalidResponse(ref msg) =>
                write!(f, "Response was invalid: {}", msg),
            Error::InvalidApiVersion { value: ref val, message: ref msg } =>
                write!(f, "{} is not a valid API version: {}", val, msg),
            Error::UnsupportedApiVersion {
                requested: ref req, minimum: minv, maximum: maxv
            } => write!(f, "Unsupported version requested: {:?}, supported \
                versions are {:?} to {:?}", req, minv, maxv)
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::EndpointNotFound(..) =>
                "Requested endpoint was not found",
            Error::InvalidInput(..) => "Invalid value(s) provided",
            Error::InvalidUrl(ref e) => e.description(),
            Error::HttpError(..) => "HTTP error",
            Error::ProtocolError(ref e) => e.description(),
            Error::InvalidResponse(..) =>
                "Invalid response received from the server",
            Error::InvalidApiVersion { .. } =>
                "Invalid API version",
            Error::UnsupportedApiVersion { .. } =>
                "Unsupported API version requested"
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::ProtocolError(ref e) => Some(e),
            Error::InvalidUrl(ref e) => Some(e),
            _ => None
        }
    }
}

impl From<HttpClientError> for Error {
    fn from(value: HttpClientError) -> Error {
        Error::ProtocolError(value)
    }
}

impl From<UrlError> for Error {
    fn from(value: UrlError) -> Error {
        Error::InvalidUrl(value)
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

fn parse_component(component: &str, value: &str, message: &str)
        -> Result<u16> {
    match component.parse() {
        Ok(val) => Ok(val),
        Err(..) => Err(Error::InvalidApiVersion {
            value: String::from(value),
            message: String::from(message)
        })
    }
}

impl FromStr for ApiVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<ApiVersion> {
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 2 {
            return Err(Error::InvalidApiVersion {
                value: String::from(s),
                message: String::from("Expected format X.Y")
            });
        }

        let major = parse_component(parts[0], s,
                                    "First component is not a number")?;

        let minor = parse_component(parts[1], s,
                                    "Second component is not a number")?;

        Ok(ApiVersion(major, minor))
    }
}

impl Serialize for ApiVersion {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

struct ApiVersionVisitor;

impl<'de> Visitor<'de> for ApiVersionVisitor {
    type Value = ApiVersion;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in format X.Y")
    }

    fn visit_str<E>(self, value: &str) -> ::std::result::Result<ApiVersion, E>
            where E: DeserError {
        ApiVersion::from_str(value).map_err(DeserError::custom)
    }
}

impl<'de> Deserialize<'de> for ApiVersion {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<ApiVersion, D::Error>
            where D: Deserializer<'de> {
        deserializer.deserialize_str(ApiVersionVisitor)
    }
}

impl<T: Into<String>> Into<(String, String)> for Sort<T> {
    fn into(self) -> (String, String) {
        match self {
            Sort::Asc(val) => (val.into(), String::from("asc")),
            Sort::Desc(val) => (val.into(), String::from("desc"))
        }
    }
}

pub mod protocol {
    #![allow(missing_docs)]

    use reqwest::Url;

    use super::super::{ApiVersion, Error, Result};
    use super::super::service::ServiceInfo;
    use super::super::utils;

    #[derive(Clone, Debug, Deserialize)]
    pub struct Link {
        pub href: String,
        pub rel: String
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Version {
        pub id: String,
        pub links: Vec<Link>,
        pub status: String,
        #[serde(deserialize_with = "utils::empty_as_none")]
        pub version: Option<ApiVersion>,
        #[serde(deserialize_with = "utils::empty_as_none")]
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
        pub fn to_service_info(&self) -> Result<ServiceInfo> {
            let endpoint = match self.links.iter().find(|x| &x.rel == "self") {
                Some(link) => Url::parse(&link.href)?,
                None => {
                    error!("Received malformed version response: no self link \
                            in {:?}", self.links);
                    return Err(
                        Error::InvalidResponse(String::from(
                                "Invalid version - missing self link"))
                    );
                }
            };

            Ok(ServiceInfo {
                root_url: endpoint,
                current_version: self.version,
                minimum_version: self.min_version
            })
        }
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
            let mut info = match serde_json::from_str::<protocol::VersionRoot>(&body) {
                Ok(ver) => ver.version.to_service_info(),
                Err(..) => {
                    // Second, assume it's a root URL.
                    let vers = resp.json::<protocol::VersionsRoot>()?;
                    match vers.versions.into_iter().find(|x| &x.id == major_version) {
                        Some(ver) => ver.to_service_info(),
                        None => Err(Error::EndpointNotFound(
                            String::from(service_type)))
                    }
                }
            }?;

            // Older Nova returns insecure URLs even for secure protocol.
            if secure {
                let _ = info.root_url.set_scheme("https").unwrap();
            }

            info!("Received {:?} from {}", info, endpoint);
            Ok(info)
        },
        Err(Error::HttpError(StatusCode::NotFound, ..)) => {
            if utils::url::is_root(&endpoint) {
                Err(Error::EndpointNotFound(String::from(service_type)))
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


#[cfg(test)]
pub mod test {
    use std::str::FromStr;

    use serde_json;

    use super::ApiVersion;

    #[test]
    fn test_apiversion_format() {
        let ver = ApiVersion(2, 27);
        assert_eq!(&ver.to_string(), "2.27");
        assert_eq!(ApiVersion::from_str("2.27").unwrap(), ver);
    }

    #[test]
    fn test_apiversion_serde() {
        let ver = ApiVersion(2, 27);
        let ser = serde_json::to_string(&ver).unwrap();
        assert_eq!(&ser, "\"2.27\"");
        assert_eq!(serde_json::from_str::<ApiVersion>(&ser).unwrap(), ver);
    }
}
