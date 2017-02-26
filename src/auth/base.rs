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

use hyper::{Result, Url};
use hyper::client::{Client, RequestBuilder};
use time::PreciseTime;


/// Authentication token.
pub struct AuthToken {
    /// Token contents.
    pub token: String,
    /// Expiration time (if any).
    pub expires_at: Option<PreciseTime>
}


/// Trait for any authentication method.
pub trait AuthMethod {
    /// Verify authentication and generate an auth token.
    ///
    /// May cache a token while it is still valid.
    fn authenticate(&mut self, client: &Client) -> Result<AuthToken>;
    /// Get a URL for the request service.
    fn get_endpoint(&mut self, service_type: &String) -> Url;
}
