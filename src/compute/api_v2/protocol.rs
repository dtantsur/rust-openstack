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

//! JSON structures and protocol bits for the Compute API.

#![allow(non_snake_case)]
#![allow(missing_docs)]

use hyper::Url;

use super::super::super::{ApiResult, ApiVersion};
use super::super::super::ApiError::MalformedResponse;
use super::super::super::service::ServiceInfo;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Server {
    pub accessIPv4: String,
    pub accessIPv6: String,
    pub id: String,
    pub name: String,
    pub status: String,
    pub tenant_id: String,
    pub user_id: String
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServersRoot {
    pub servers: Vec<ServerSummary>
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerRoot {
    pub server: Server
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Link {
    pub href: String,
    pub rel: String
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Version {
    pub id: String,
    pub links: Vec<Link>,
    pub status: String,
    pub version: String,
    pub min_version: String
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionsRoot {
    pub versions: Vec<Version>
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionRoot {
    pub version: Version
}


impl Version {
    pub fn into_service_info(self) -> ApiResult<ServiceInfo> {
        let current_version = if self.version.is_empty() {
            None
        } else {
            Some(try!(ApiVersion::parse(self.version)))
        };

        let minimum_version = if self.min_version.is_empty() {
            None
        } else {
            Some(try!(ApiVersion::parse(self.min_version)))
        };

        let endpoint = match self.links.iter().find(|x| &x.rel == "self") {
            Some(link) => try!(Url::parse(&link.href)),
            None => {
                let msg = format!("No link to self in version, only {:?}",
                                  self.links);
                return Err(MalformedResponse(msg))
            }
        };

        Ok(ServiceInfo {
            root_url: endpoint,
            current_version: current_version,
            minimum_version: minimum_version
        })
    }
}
