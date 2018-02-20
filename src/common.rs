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
use std::time::{Duration, Instant};
use std::thread::sleep;

use reqwest::{Method, StatusCode, Url, UrlError};
use reqwest::Error as HttpClientError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error as DeserError, Visitor};
use serde_json;

use super::auth::AuthMethod;
use super::service::ServiceInfo;
use super::utils;


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

/// Trait representing a waiter for some asynchronous action to finish.
///
/// The type `T` is the final type of the action, while type `P` represents
/// an intermediate state.
pub trait Waiter<T, P=T> {
    /// Update the current state of the action.
    ///
    /// Returns `T` if the action is finished, `None` if it is not. All errors
    /// are propagated via the `Result`.
    ///
    /// This method should not be called again after it returned the final
    /// result.
    fn poll(&mut self) -> Result<Option<T>>;

    /// Default timeout for this action.
    ///
    /// This timeout is used in the `wait` method.
    /// If `None, wait forever by default.
    fn default_wait_timeout(&self) -> Option<Duration> { None }
    /// Default delay between two retries.
    ///
    /// The default is 0.1 seconds and should be changed by implementations.
    fn default_delay(&self) -> Duration {
        Duration::from_millis(100)
    }
    /// Error message to return on time out.
    fn timeout_error_message(&self) -> String {
        "Timeout while waiting for operation to finish".to_string()
    }

    /// Wait for the default amount of time.
    ///
    /// Returns `OperationTimedOut` if the timeout is reached.
    fn wait(self) -> Result<T> where Self: Sized {
        match self.default_wait_timeout() {
            Some(duration) => self.wait_for(duration),
            None => self.wait_forever()
        }
    }
    /// Wait for specified amount of time.
    ///
    /// Returns `OperationTimedOut` if the timeout is reached.
    fn wait_for(self, duration: Duration) -> Result<T> where Self: Sized{
        let delay = self.default_delay();
        self.wait_for_with_delay(duration, delay)
    }
    /// Wait for specified amount of time.
    ///
    /// Returns `OperationTimedOut` if the timeout is reached.
    fn wait_for_with_delay(mut self, duration: Duration, delay: Duration)
            -> Result<T> where Self: Sized {
        let start = Instant::now();
        while Instant::now().duration_since(start) <= duration {
            match self.poll()? {
                Some(result) => return Ok(result),
                None => ()  // continue
            };
            sleep(delay);
        };
        Err(Error::new(ErrorKind::OperationTimedOut,
                       self.timeout_error_message()))
    }
    /// Wait forever.
    fn wait_forever(self) -> Result<T> where Self: Sized {
        let delay = self.default_delay();
        self.wait_forever_with_delay(delay)
    }
    /// Wait forever with given delay between attempts.
    fn wait_forever_with_delay(mut self, delay: Duration)
            -> Result<T> where Self: Sized {
        loop {
            match self.poll()? {
                Some(result) => return Ok(result),
                None => ()  // continue
            };
            sleep(delay);
        }
    }
}


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

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

fn parse_component(component: &str, message: &str) -> Result<u16> {
    component.parse().map_err(|_| {
        Error::new(ErrorKind::InvalidResponse, message)
    })
}

impl FromStr for ApiVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<ApiVersion> {
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 2 {
            let msg = format!("Invalid API version: expected X.Y, got {}", s);
            return Err(Error::new(ErrorKind::InvalidResponse, msg))
        }

        let major = parse_component(parts[0],
                                    "First version component is not a number")?;

        let minor = parse_component(parts[1],
                                    "Second version component is not a number")?;

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

    use super::super::{ApiVersion, Error, ErrorKind, Result};
    use super::super::service::ServiceInfo;
    use super::super::utils;

    #[derive(Clone, Debug, Deserialize)]
    pub struct Link {
        pub href: String,
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
                        None => Err(Error::new_endpoint_not_found(service_type))
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
