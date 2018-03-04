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

//! Error and Result implementations.

use std::fmt;

use reqwest::{StatusCode, UrlError};
use reqwest::Error as HttpClientError;

/// Kind of an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Authentication failure
    ///
    /// Maps to HTTP 401.
    AuthenticationFailed,

    /// Access denied.
    ///
    /// Maps to HTTP 403.
    AccessDenied,

    /// Requested resource was not found.
    ///
    /// Roughly maps to HTTP 404 and 410.
    ResourceNotFound,

    /// Request returned more items than expected.
    TooManyItems,

    /// Requested service endpoint was not found.
    EndpointNotFound,

    /// Invalid value passed to one of paremeters.
    ///
    /// May be result of HTTP 400.
    InvalidInput,

    /// Unsupported or incompatible API version.
    ///
    /// May be a result of HTTP 406.
    IncompatibleApiVersion,

    /// Conflict in the request.
    Conflict,

    /// Operation has reached the specified time out.
    OperationTimedOut,

    /// Operation failed to complete.
    OperationFailed,

    /// Protocol-level error reported by underlying HTTP library.
    ProtocolError,

    /// Response received from the server is malformed.
    InvalidResponse,

    /// Internal server error.
    ///
    /// Maps to HTTP 5xx codes.
    InternalServerError,

    #[allow(missing_docs)]
    __Nonexhaustive,
}

/// Error from an OpenStack call.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    status: Option<StatusCode>,
    message: Option<String>
}

/// Result of an OpenStack call.
pub type Result<T> = ::std::result::Result<T, Error>;


impl Error {
    pub(crate) fn new<S: Into<String>>(kind: ErrorKind, message: S) -> Error {
        Error {
            kind: kind,
            status: None,
            message: Some(message.into())
        }
    }

    /// Create with providing all details.
    pub(crate) fn new_with_details(kind: ErrorKind, status: Option<StatusCode>,
                                   message: Option<String>) -> Error {
        Error {
            kind: kind,
            status: status,
            message: message
        }
    }

    /// Error kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Helper - error of kind EndpointNotFound.
    pub(crate) fn new_endpoint_not_found<D: fmt::Display>(service_type: D) -> Error {
        Error::new(
            ErrorKind::EndpointNotFound,
            format!("Endpoint for service {} was not found", service_type)
        )
    }
}

impl ErrorKind {
    /// Short description of the error kind.
    pub fn description(&self) -> &'static str {
        match self {
            &ErrorKind::AuthenticationFailed =>
                "Failed to authenticate",
            &ErrorKind::AccessDenied =>
                "Access to the resource is denied",
            &ErrorKind::ResourceNotFound =>
                "Requested resource was not found",
            &ErrorKind::TooManyItems =>
                "Request returned too many items",
            &ErrorKind::EndpointNotFound =>
                "Requested endpoint was not found",
            &ErrorKind::InvalidInput =>
                "Input value(s) are invalid or missing",
            &ErrorKind::IncompatibleApiVersion =>
                "Incompatible or unsupported API version",
            &ErrorKind::Conflict =>
                "Requested cannot be fulfilled due to a conflict",
            &ErrorKind::OperationTimedOut =>
                "Time out reached while waiting for the operation",
            &ErrorKind::OperationFailed =>
                "Requested operation has failed",
            &ErrorKind::ProtocolError =>
                "Error when accessing the server",
            &ErrorKind::InvalidResponse =>
                "Received invalid response",
            &ErrorKind::InternalServerError =>
                "Internal server error or bad gateway",
            _ => unreachable!()
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(ref msg) = self.message {
            write!(f, ": {}", msg)
        } else {
            Ok(())
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        self.kind.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}

impl From<HttpClientError> for Error {
    fn from(value: HttpClientError) -> Error {
        let msg = value.to_string();
        let kind = match value.status() {
            Some(StatusCode::Unauthorized) => ErrorKind::AuthenticationFailed,
            Some(StatusCode::Forbidden) => ErrorKind::AccessDenied,
            Some(StatusCode::NotFound) => ErrorKind::ResourceNotFound,
            Some(StatusCode::NotAcceptable) => ErrorKind::IncompatibleApiVersion,
            Some(StatusCode::Conflict) => ErrorKind::Conflict,
            Some(c) if c.is_client_error() => ErrorKind::InvalidInput,
            Some(c) if c.is_server_error() => ErrorKind::InternalServerError,
            None => ErrorKind::ProtocolError,
            _ => ErrorKind::InvalidResponse
        };

        Error::new_with_details(kind, value.status(), Some(msg))
    }
}

impl From<UrlError> for Error {
    fn from(value: UrlError) -> Error {
        Error::new(ErrorKind::InvalidInput, value.to_string())
    }
}

