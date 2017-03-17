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

use hyper::Error as HttpClientError;
use hyper::client::Response;
use hyper::error::ParseError;
use hyper::status::StatusCode;
use serde_json::Error as JsonError;


/// Error from an OpenStack API call.
#[derive(Debug)]
pub enum ApiError {
    /// Insufficient credentials passed to make authentication request.
    ///
    /// Contains the error message.
    InsufficientCredentials(String),

    /// Requested service endpoint was not found.
    ///
    /// Contains the failed endpoint name.
    EndpointNotFound(String),

    /// Invalid value passed to one of paremeters.
    ///
    /// Contains the error message.
    InvalidParameterValue(String),

    /// Generic HTTP error.
    HttpError(StatusCode, Response),

    /// Protocol-level error reported by underlying HTTP library.
    ProtocolError(HttpClientError),

    /// JSON parsing failed.
    InvalidJson(JsonError),

    /// Malformed response.
    MalformedResponse(String),

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
            ApiError::InsufficientCredentials(ref msg) =>
                write!(f, "Insufficient credentials provided: {}", msg),
            ApiError::EndpointNotFound(ref endp) =>
                write!(f, "Requested endpoint {} was not found", endp),
            ApiError::InvalidParameterValue(ref msg) =>
                write!(f, "Passed parameters are invalid: {}", msg),
            ApiError::HttpError(status, ..) =>
                write!(f, "HTTP error {}", status),
            ApiError::ProtocolError(ref e) => fmt::Display::fmt(e, f),
            ApiError::InvalidJson(ref e) => fmt::Display::fmt(e, f),
            ApiError::MalformedResponse(ref msg) =>
                write!(f, "Malformed response received: {}", msg),
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
            ApiError::InsufficientCredentials(..) =>
                "Insufficient credentials provided",
            ApiError::EndpointNotFound(..) =>
                "Requested endpoint was not found",
            ApiError::InvalidParameterValue(..) =>
                "Invalid values passed for parameters",
            ApiError::HttpError(..) => "HTTP error",
            ApiError::ProtocolError(ref e) => e.description(),
            ApiError::InvalidJson(ref e) => e.description(),
            ApiError::MalformedResponse(..) =>
                "Malformed response received",
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

fn parse_component(component: &str, value: &String, message: &str)
        -> ApiResult<u16> {
    match component.parse() {
        Ok(val) => Ok(val),
        Err(..) => Err(ApiError::InvalidApiVersion {
            value: value.clone(),
            message: String::from(message)
        })
    }
}

impl ApiVersion {
    /// Parse string, yielding an API version.
    pub fn parse<S: Into<String>>(value: S) -> ApiResult<ApiVersion> {
        let s = value.into();
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 2 {
            return Err(ApiError::InvalidApiVersion {
                value: s.clone(),
                message: String::from("Expected format X.Y")
            });
        }

        let major = try!(parse_component(parts[0], &s,
                                         "First component is not a number"));

        let minor = try!(parse_component(parts[1], &s,
                                         "Second component is not a number"));

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
