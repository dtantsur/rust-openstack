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

//! Base code for authentication.

#![allow(missing_docs)]

use std::fmt;

use hyper::{Client, Url};
use hyper::client::IntoUrl;
use hyper::error::ParseError;
use time::PreciseTime;

use super::super::{ApiError, Session};

header! { (AuthTokenHeader, "X-Auth-Token") => [String] }

header! { (SubjectTokenHeader, "X-Subject-Token") => [String] }


/// Trait for authentication token implementations.
pub trait AuthToken: Clone + fmt::Debug + Send + Into<String> {
    /// A reference to token contents.
    fn value(&self) -> &String;
    /// Expiration time (if any).
    fn expires_at(&self) -> Option<&PreciseTime>;
}


/// Simple authentication token.
#[derive(Clone, Debug)]
pub struct SimpleAuthToken(pub String);

/// Trait for any authentication method.
pub trait AuthMethod: Clone + Send {
    /// A token type.
    type TokenType: AuthToken;

    /// Verify authentication and generate an auth token.
    fn get_token(&self, client: &Client)
        -> Result<Self::TokenType, ApiError>;

    /// Get a URL for the requested service.
    fn get_endpoint(&self, service_type: &str,
                    endpoint_interface: Option<&str>,
                    region: Option<&str>,
                    session: &Session<Self>)
        -> Result<Url, ApiError>;
}

/// Authentication method that provides no authentication (uses a fake token).
#[derive(Clone, Debug)]
pub struct NoAuth {
    endpoint: Url
}

impl Into<String> for SimpleAuthToken {
    fn into(self) -> String {
        self.0
    }
}

impl AuthToken for SimpleAuthToken {
    fn value(&self) -> &String {
        &self.0
    }

    fn expires_at(&self) -> Option<&PreciseTime> {
        None
    }
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
    type TokenType = SimpleAuthToken;

    /// Return a fake token for compliance with the protocol.
    fn get_token(&self, _client: &Client) -> Result<SimpleAuthToken, ApiError> {
        Ok(SimpleAuthToken(String::from("no-auth")))
    }

    /// Get a predefined endpoint for all service types
    fn get_endpoint(&self, _service_type: &str,
                    _endpoint_interface: Option<&str>,
                    _region: Option<&str>,
                    _client: &Session<NoAuth>)
            -> Result<Url, ApiError> {
        Ok(self.endpoint.clone())
    }
}

#[cfg(test)]
pub mod test {
    use hyper;

    use super::super::super::session::test::new_session;

    use super::{AuthMethod, AuthToken, NoAuth};

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
        let a = NoAuth::new("http://127.0.0.1:8080/v1").unwrap();
        let tok = a.get_token(&hyper::Client::new()).unwrap();
        assert_eq!(tok.value(), "no-auth");
        assert!(tok.expires_at().is_none());
    }

    #[test]
    fn test_noauth_get_endpoint() {
        let a = NoAuth::new("http://127.0.0.1:8080/v1").unwrap();
        let e = a.get_endpoint("foobar", None, None,
                               &new_session("token")).unwrap();
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/v1");
    }
}
