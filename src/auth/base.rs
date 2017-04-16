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

use hyper::{Client, Url};

use super::super::{ApiResult, Session};


/// Trait for an authentication method.
///
/// An OpenStack authentication method is expected to be able to:
///
/// 1. get an authentication token to use when accessing services,
/// 2. get an endpoint URL for the given service type.
///
/// An authentication method should cache the token as long as it's valid.
pub trait Method: Sized {
    /// Default endpoint interface that is used when none is provided.
    fn default_endpoint_interface(&self) -> String {
        String::from("public")
    }

    /// Default region to use with this authentication (if any).
    fn default_region(&self) -> Option<String> { None }

    /// Verify authentication and generate an auth token.
    ///
    /// An authentication method should cache the token as long as it's valid.
    fn get_token(&self, client: &Client) -> ApiResult<String>;

    /// Get a URL for the requested service.
    fn get_endpoint(&self, service_type: String,
                    endpoint_interface: Option<String>,
                    region: Option<String>,
                    session: &Session<Self>) -> ApiResult<Url>;

    /// Create a session with this authentication method.
    fn session(self) -> Session<Self> {
        Session::new(self)
    }
}
