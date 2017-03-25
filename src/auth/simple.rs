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

//! Simple authentication methods.

use std::fmt;

use hyper::{Client, Url};
use hyper::client::IntoUrl;
use hyper::error::ParseError;

use super::super::{ApiResult, Session};
use super::{Method, Token};

/// Plain authentication token without additional details.
#[derive(Clone, Debug)]
pub struct SimpleToken(pub String);

/// Authentication method that provides no authentication.
///
/// This method always returns a constant fake token, and a pre-defined
/// endpoint.
#[derive(Clone, Debug)]
pub struct NoAuth {
    endpoint: Url
}

impl fmt::Display for SimpleToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Into<String> for SimpleToken {
    fn into(self) -> String {
        self.0
    }
}

impl Token for SimpleToken {
    fn value(&self) -> &String {
        &self.0
    }

    fn needs_refresh(&self) -> bool {
        false
    }
}

impl NoAuth {
    /// Create a new fake authentication method using a fixed endpoint.
    ///
    /// This endpoint will be returned in response to all get_endpoint calls
    /// of the [Method](trait.Method.html) trait.
    pub fn new<U>(endpoint: U) -> Result<NoAuth, ParseError> where U: IntoUrl {
        let url = try!(endpoint.into_url());
        Ok(NoAuth {
            endpoint: url
        })
    }
}

impl Method for NoAuth {
    type TokenType = SimpleToken;

    /// Return a fake token for compliance with the protocol.
    fn get_token(&self, _client: &Client) -> ApiResult<SimpleToken> {
        Ok(SimpleToken(String::from("no-auth")))
    }

    /// Get a predefined endpoint for all service types
    fn get_endpoint(&self, _service_type: String,
                    _endpoint_interface: Option<String>,
                    _region: Option<String>,
                    _client: &Session<NoAuth>) -> ApiResult<Url> {
        Ok(self.endpoint.clone())
    }
}

#[cfg(test)]
pub mod test {
    #![allow(unused_results)]

    use hyper;

    use super::super::super::session::test::new_session;
    use super::super::{Method, Token};
    use super::NoAuth;

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
        assert!(tok.valid());
        assert!(!tok.needs_refresh());
    }

    #[test]
    fn test_noauth_get_endpoint() {
        let a = NoAuth::new("http://127.0.0.1:8080/v1").unwrap();
        let e = a.get_endpoint(String::from("foobar"), None, None,
                               &new_session("token")).unwrap();
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/v1");
    }
}
