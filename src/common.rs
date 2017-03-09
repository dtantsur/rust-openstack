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


/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Version suffix to append to the endpoint.
    fn version_suffix() -> Option<&'static str>;
}


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
    InvalidJson(JsonError)
}

/// Result of an API call.
pub type ApiResult<T> = Result<T, ApiError>;

/// API version (major, minor).
#[derive(Copy, Clone, Debug)]
pub struct ApiVersion(pub u16, pub u16);


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
            ApiError::InvalidJson(ref e) => fmt::Display::fmt(e, f)
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
            ApiError::InvalidJson(ref e) => e.description()
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

#[cfg(test)]
pub mod test {
    use super::ApiVersion;

    #[test]
    fn test_apiversion() {
        let ver = ApiVersion(2, 27);
        assert_eq!(&ver.to_string(), "2.27");
    }
}
