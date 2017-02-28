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

//! Low-level code to work with the service catalog.

use hyper::{Get, Url};

use super::super::ApiError;
use super::super::session::AuthenticatedClient;
use super::protocol;

/// Fetch the service catalog from a given auth URL.
pub fn get_service_catalog(auth_url: &Url, client: &AuthenticatedClient)
        -> Result<Vec<protocol::CatalogRecord>, ApiError> {
    let url = format!("{}/v3/auth/catalog", auth_url.to_string());
    debug!("Requesting a service catalog from {}", url);

    let resp = try!(client.request(Get, &url).send());
    let body = try!(protocol::CatalogRoot::from_reader(resp));
    Ok(body.catalog)
}
