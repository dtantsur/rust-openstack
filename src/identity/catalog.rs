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

/// Type alias for the catalog.
pub type Catalog = Vec<protocol::CatalogRecord>;

/// Fetch the service catalog from a given auth URL.
pub fn get_service_catalog(auth_url: &Url, client: &AuthenticatedClient)
        -> Result<Catalog, ApiError> {
    let url = format!("{}/v3/auth/catalog", auth_url.to_string());
    debug!("Requesting a service catalog from {}", url);

    let resp = try!(client.request(Get, &url).send());
    let body = try!(protocol::CatalogRoot::from_reader(resp));
    Ok(body.catalog)
}

/// Find an endpoint in the service catalog.
pub fn find_endpoint<'a>(catalog: &'a Catalog, service_type: &str,
                         endpoint_interface: &str, region: Option<&str>)
        -> Result<&'a protocol::Endpoint, ApiError> {
    let svc = match catalog.iter().find(|x| &x.service_type == service_type) {
        Some(s) => s,
        None => return Err(ApiError::EndpointNotFound)
    };

    let maybe_endp: Option<&protocol::Endpoint>;
    if let Some(rgn) = region {
        maybe_endp = svc.endpoints.iter().find(
            |x| &x.interface == endpoint_interface && &x.region == rgn);
    } else {
        maybe_endp = svc.endpoints.iter().find(
            |x| &x.interface == endpoint_interface);
    }

    match maybe_endp {
        Some(e) => Ok(e),
        None => Err(ApiError::EndpointNotFound)
    }
}
