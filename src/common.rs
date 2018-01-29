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

use reqwest::{Response, StatusCode, UrlError};
use reqwest::Error as HttpClientError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error as DeserError, Visitor};


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
