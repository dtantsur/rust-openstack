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
use hyper::status::StatusCode;
use serde_json::Error as JsonError;


/// Error from an OpenStack API call.
#[derive(Debug)]
pub enum ApiError {
    /// Insufficient credentials passed to make authentication request.
    InsufficientCredentials(&'static str),
    /// Requested service endpoint was not found.
    EndpointNotFound,
    /// Authentication rejected (invalid credentials or token).
    Unauthorized,
    /// Generic HTTP error (not covered by EndpointNotFound and Unauthorized).
    HttpError(StatusCode, Option<String>),
    /// Protocol-level error reported by underlying HTTP library.
    ProtocolError(HttpClientError),
    /// JSON parsing failed.
    InvalidJson(JsonError)
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ApiError::InsufficientCredentials(msg) =>
                write!(f, "Insufficient credentials provided: {}", msg),
            ApiError::EndpointNotFound =>
                write!(f, "Requested endpoint was not found"),
            ApiError::Unauthorized =>
                write!(f, "Authentication failed"),
            ApiError::HttpError(status, ref maybe_msg) =>
                match *maybe_msg {
                    Some(ref msg) =>
                        write!(f, "HTTP error {}: {}", status, msg),
                    None =>
                        write!(f, "HTTP error {}", status),
                },
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
            ApiError::EndpointNotFound => "Requested endpoint was not found",
            ApiError::Unauthorized => "Authentication failed",
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
