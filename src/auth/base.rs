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

use std::fmt;

use hyper::{Client, Url};
use time::PreciseTime;

use super::super::{ApiResult, Session};


/// Trait for authentication token implementations.
pub trait Token: Clone + fmt::Debug + Send + fmt::Display + Into<String> {
    /// A reference to token contents.
    fn value(&self) -> &String;

    /// Expiration time (if any).
    fn expires_at(&self) -> Option<&PreciseTime>;
}

/// Trait for an authentication method.
///
/// An OpenStack authentication method is expected to be able to:
///
/// 1. get an authentication token to use when accessing services,
/// 2. get an endpoint URL for the given service type.
pub trait Method: Clone + Send {
    /// A token type.
    type TokenType: Token;

    /// Verify authentication and generate an auth token.
    fn get_token(&self, client: &Client) -> ApiResult<Self::TokenType>;

    /// Get a URL for the requested service.
    fn get_endpoint(&self, service_type: &str,
                    endpoint_interface: Option<&str>,
                    region: Option<&str>,
                    session: &Session<Self>) -> ApiResult<Url>;

    /// Create a session with this authentication method.
    fn session(self) -> Session<Self> {
        Session::new(self)
    }
}
