// Copyright 2016 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Base code for authentication.

use std::error::Error;
use std::fmt;
use std::io;

use hyper::{Client, Url};
use hyper::Error as HttpClientError;
use hyper::client::IntoUrl;
use hyper::error::ParseError;
use hyper::status::StatusCode;
use time::PreciseTime;


/// Authentication token.
#[derive(Clone)]
pub struct AuthToken {
    /// Token contents.
    pub token: String,
    /// Expiration time (if any).
    pub expires_at: Option<PreciseTime>
}

header! { (AuthTokenHeader, "X-Auth-Token") => [String] }
header! { (SubjectTokenHeader, "X-Subject-Token") => [String] }

/// Error from an authentication call.
#[derive(Debug)]
pub enum AuthError {
    /// Insufficient credentials passed to make authentication request.
    InsufficientCredentials(&'static str),
    /// Requested service endpoint was not found.
    EndpointNotFound,
    /// Authentication rejected (invalid credentials or token).
    Unauthorized,
    /// Generic HTTP error (not covered by EndpointNotFound and Unauthorized).
    HttpError(StatusCode, Option<String>),
    /// Protocol-level error reported by underlying HTTP library.
    ProtocolError(HttpClientError)
}

/// Trait for any authentication method.
pub trait AuthMethod {
    /// Verify authentication and generate an auth token.
    ///
    /// May cache a token while it is still valid.
    fn get_token(&mut self, client: &Client) -> Result<AuthToken, AuthError>;
    /// Get a URL for the request service.
    fn get_endpoint(&mut self, service_type: &str, client: &Client)
        -> Result<Url, AuthError>;
}

/// Authentication method that provides no authentication (uses a fake token).
pub struct NoAuth {
    endpoint: Url
}

impl NoAuth {
    /// Create a new fake authentication method using a fixed endpoint.
    pub fn new<U>(endpoint: U) -> Result<NoAuth, ParseError> where U: IntoUrl {
        let url = try!(endpoint.into_url());
        Ok(NoAuth {
            endpoint: url
        })
    }
}

impl AuthMethod for NoAuth {
    /// Return a fake token for compliance with the protocol.
    fn get_token(&mut self, _client: &Client) -> Result<AuthToken, AuthError> {
        Ok(AuthToken {
            token: String::from("no-auth"),
            expires_at: None
        })
    }

    /// Get a predefined endpoint for all service types
    fn get_endpoint(&mut self, _service_type: &str, _client: &Client)
            -> Result<Url, AuthError> {
        Ok(self.endpoint.clone())
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AuthError::InsufficientCredentials(msg) =>
                write!(f, "Insufficient credentials provided: {}", msg),
            AuthError::EndpointNotFound =>
                write!(f, "Requested endpoint was not found"),
            AuthError::Unauthorized =>
                write!(f, "Authentication failed"),
            AuthError::HttpError(status, ref maybe_msg) =>
                match *maybe_msg {
                    Some(ref msg) =>
                        write!(f, "HTTP error {}: {}", status, msg),
                    None =>
                        write!(f, "HTTP error {}", status),
                },
            AuthError::ProtocolError(ref e) => fmt::Display::fmt(e, f)
        }
    }
}

impl Error for AuthError {
    fn description(&self) -> &str {
        match *self {
            AuthError::InsufficientCredentials(..) =>
                "Insufficient credentials provided",
            AuthError::EndpointNotFound => "Requested endpoint was not found",
            AuthError::Unauthorized => "Authentication failed",
            AuthError::HttpError(..) => "HTTP error",
            AuthError::ProtocolError(ref e) => e.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            AuthError::ProtocolError(ref e) => Some(e),
            _ => None
        }
    }
}

impl From<HttpClientError> for AuthError {
    fn from(value: HttpClientError) -> AuthError {
        AuthError::ProtocolError(value)
    }
}

impl From<io::Error> for AuthError {
    fn from(value: io::Error) -> AuthError {
        AuthError::ProtocolError(HttpClientError::Io(value))
    }
}

#[cfg(test)]
pub mod test {
    use hyper;

    use super::{AuthMethod, NoAuth};

    #[test]
    fn test_noauth_new() {
        let a = NoAuth::new("http://127.0.0.1:8080/v1").unwrap();
        let e = a.endpoint;
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/v1");
    }

    #[test]
    fn test_noauth_new_fail() {
        NoAuth::new("foo bar").err().unwrap();
    }

    #[test]
    fn test_noauth_get_token() {
        let mut a = NoAuth::new("http://127.0.0.1:8080/v1").unwrap();
        let tok = a.get_token(&hyper::Client::new()).unwrap();
        assert_eq!(&tok.token, "no-auth");
        assert!(tok.expires_at.is_none());
    }

    #[test]
    fn test_noauth_get_endpoint() {
        let mut a = NoAuth::new("http://127.0.0.1:8080/v1").unwrap();
        let e = a.get_endpoint("foobar", &hyper::Client::new()).unwrap();
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/v1");
    }
}
