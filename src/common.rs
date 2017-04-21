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

use std::error::Error;
use std::fmt;
use std::io;
use std::str::FromStr;

use hyper::Error as HttpClientError;
use hyper::client::Response;
use hyper::error::ParseError;
use hyper::status::StatusCode;
use serde_json::Error as JsonError;


/// Error from an OpenStack API call.
#[derive(Debug)]
pub enum ApiError {
    /// Requested service endpoint was not found.
    ///
    /// Contains the failed endpoint name.
    EndpointNotFound(String),

    /// Invalid value passed to one of paremeters.
    ///
    /// Contains the error message.
    InvalidInput(String),

    /// Generic HTTP error.
    HttpError(StatusCode, Response),

    /// Protocol-level error reported by underlying HTTP library.
    ProtocolError(HttpClientError),

    /// JSON parsing failed.
    InvalidJson(JsonError),

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

/// Result of an API call.
pub type ApiResult<T> = Result<T, ApiError>;

/// API version (major, minor).
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct ApiVersion(pub u16, pub u16);

/// A request for negotiating an API version.
#[derive(Debug, Clone)]
pub enum ApiVersionRequest {
    /// Minimum possible version (usually the default).
    Minimum,
    /// Latest version.
    Latest,
    /// Specified version.
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


impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ApiError::EndpointNotFound(ref endp) =>
                write!(f, "Requested endpoint {} was not found", endp),
            ApiError::InvalidInput(ref msg) =>
                write!(f, "Input value(s) are invalid: {}", msg),
            ApiError::HttpError(status, ..) =>
                write!(f, "HTTP error {}", status),
            ApiError::ProtocolError(ref e) => fmt::Display::fmt(e, f),
            ApiError::InvalidJson(ref e) => fmt::Display::fmt(e, f),
            ApiError::InvalidApiVersion { value: ref val, message: ref msg } =>
                write!(f, "{} is not a valid API version: {}", val, msg),
            ApiError::UnsupportedApiVersion {
                requested: ref req, minimum: minv, maximum: maxv
            } => write!(f, "Unsupported version requested: {:?}, supported \
                versions are {:?} to {:?}", req, minv, maxv)
        }
    }
}

impl Error for ApiError {
    fn description(&self) -> &str {
        match *self {
            ApiError::EndpointNotFound(..) =>
                "Requested endpoint was not found",
            ApiError::InvalidInput(..) => "Invalid value(s) provided",
            ApiError::HttpError(..) => "HTTP error",
            ApiError::ProtocolError(ref e) => e.description(),
            ApiError::InvalidJson(ref e) => e.description(),
            ApiError::InvalidApiVersion { .. } =>
                "Invalid API version",
            ApiError::UnsupportedApiVersion { .. } =>
                "Unsupported API version requested"
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            ApiError::ProtocolError(ref e) => Some(e),
            ApiError::InvalidJson(ref e) => Some(e),
            _ => None
        }
    }
}

impl From<HttpClientError> for ApiError {
    fn from(value: HttpClientError) -> ApiError {
        ApiError::ProtocolError(value)
    }
}

impl From<io::Error> for ApiError {
    fn from(value: io::Error) -> ApiError {
        ApiError::ProtocolError(HttpClientError::Io(value))
    }
}

impl From<JsonError> for ApiError {
    fn from(value: JsonError) -> ApiError {
        ApiError::InvalidJson(value)
    }
}

impl From<ParseError> for ApiError {
    fn from(value: ParseError) -> ApiError {
        ApiError::ProtocolError(HttpClientError::Uri(value))
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

fn parse_component(component: &str, value: &str, message: &str)
        -> ApiResult<u16> {
    match component.parse() {
        Ok(val) => Ok(val),
        Err(..) => Err(ApiError::InvalidApiVersion {
            value: String::from(value),
            message: String::from(message)
        })
    }
}

impl FromStr for ApiVersion {
    type Err = ApiError;

    fn from_str(s: &str) -> ApiResult<ApiVersion> {
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 2 {
            return Err(ApiError::InvalidApiVersion {
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
    use super::ApiVersion;

    #[test]
    fn test_apiversion() {
        let ver = ApiVersion(2, 27);
        assert_eq!(&ver.to_string(), "2.27");
    }
}
