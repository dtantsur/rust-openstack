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

//! Foundation bits exposing the Compute API.

use reqwest::{Method, Response, StatusCode, Url};
use reqwest::header::Headers;
use serde_json;

use super::super::super::{ApiResult, ApiVersion};
use super::super::super::ApiError::{HttpError, EndpointNotFound};
use super::super::super::auth::AuthMethod;
use super::super::super::service::{ApiVersioning, ServiceInfo, ServiceType,
                                   ServiceWrapper};
use super::super::super::utils;
use super::protocol::{VersionRoot, VersionsRoot};


/// Service wrapper for Compute API V2.
pub type V2ServiceWrapper<'session> = ServiceWrapper<'session, V2>;

/// Service type of Compute API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "compute";
const VERSION_ID: &'static str = "v2.1";

fn extract_info(mut resp: Response, secure: bool) -> ApiResult<ServiceInfo> {
    let body = resp.text()?;

    // First, assume it's a versioned URL.
    let mut info = match serde_json::from_str::<VersionRoot>(&body) {
        Ok(ver) => ver.version.to_service_info(),
        Err(..) => {
            // Second, assume it's a root URL.
            let vers: VersionsRoot = resp.json()?;
            match vers.versions.into_iter().find(|x| &x.id == VERSION_ID) {
                Some(ver) => ver.to_service_info(),
                None => Err(EndpointNotFound(String::from(SERVICE_TYPE)))
            }
        }
    }?;

    // Nova returns insecure URLs even for secure protocol. WHY??
    if secure {
        let _ = info.root_url.set_scheme("https").unwrap();
    }

    Ok(info)
}

impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod)
            -> ApiResult<ServiceInfo> {
        debug!("Fetching compute service info from {}", endpoint);
        let secure = endpoint.scheme() == "https";
        let result = auth.request(Method::Get, endpoint.clone())?.send()
            .map_err(From::from);
        match result {
            Ok(resp) => {
                let result = extract_info(resp, secure)?;
                info!("Received {:?} from {}", result, endpoint);
                Ok(result)
            },
            Err(HttpError(StatusCode::NotFound, ..)) => {
                if utils::url::is_root(&endpoint) {
                    Err(EndpointNotFound(String::from(SERVICE_TYPE)))
                } else {
                    debug!("Got HTTP 404 from {}, trying parent endpoint",
                           endpoint);
                    V2::service_info(
                        utils::url::pop(endpoint, true),
                        auth)
                }
            },
            Err(other) => Err(other)
        }
    }

    fn api_version_headers(version: ApiVersion) -> Option<Headers> {
        let mut hdrs = Headers::new();
        // TODO: typed header, new-style header support
        hdrs.set_raw("x-openstack-nova-api-version", version.to_string());
        Some(hdrs)
    }
}

impl ApiVersioning for V2 {}
