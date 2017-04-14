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

use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use hyper::Url;
use serde::de::Error as DeserError;
use serde_json::Error as JsonError;

use super::super::super::{ApiResult, ApiVersion};
use super::super::super::ApiError::InvalidJson;
use super::super::super::service::ServiceInfo;
use super::super::super::utils;


/// Available sort keys.
#[derive(Debug, Copy, Clone)]
pub enum ServerSortKey {
    AccessIpv4,
    AccessIpv6,
    AutoDiskConfig,
    AvailabilityZone,
    ConfigDrive,
    CreatedAt,
    DisplayDescription,
    DisplayName,
    Host,
    HostName,
    ImageRef,
    InstanceTypeId,
    KernelId,
    KeyName,
    LaunchIndex,
    LaunchedAt,
    LockedBy,
    Node,
    PowerState,
    Progress,
    ProjectId,
    RamdiskId,
    RootDeviceName,
    TaskState,
    TerminatedAt,
    UpdatedAt,
    UserId,
    Uuid,
    VmState,
    #[doc(hidden)]
    __Nonexhaustive,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Server {
    #[serde(deserialize_with = "utils::empty_as_none")]
    pub accessIPv4: Option<Ipv4Addr>,
    #[serde(deserialize_with = "utils::empty_as_none")]
    pub accessIPv6: Option<Ipv6Addr>,
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
    pub fn to_service_info(&self) -> ApiResult<ServiceInfo> {
        let current_version = if self.version.is_empty() {
            None
        } else {
            Some(try!(ApiVersion::from_str(&self.version)))
        };

        let minimum_version = if self.min_version.is_empty() {
            None
        } else {
            Some(try!(ApiVersion::from_str(&self.min_version)))
        };

        let endpoint = match self.links.iter().find(|x| &x.rel == "self") {
            Some(link) => try!(Url::parse(&link.href)),
            None => {
                error!("Received malformed version response: no self link \
                        in {:?}", self.links);
                return Err(
                    InvalidJson(JsonError::missing_field("link to self"))
                );
            }
        };

        Ok(ServiceInfo {
            root_url: endpoint,
            current_version: current_version,
            minimum_version: minimum_version
        })
    }
}

impl Into<String> for ServerSortKey {
    fn into(self) -> String {
        String::from(match self {
            ServerSortKey::AccessIpv4 => "access_ip_v4",
            ServerSortKey::AccessIpv6 => "access_ip_v6",
            ServerSortKey::AutoDiskConfig => "auto_disk_config",
            ServerSortKey::AvailabilityZone => "availability_zone",
            ServerSortKey::ConfigDrive => "config_drive",
            ServerSortKey::CreatedAt => "created_at",
            ServerSortKey::DisplayDescription => "display_description",
            ServerSortKey::DisplayName => "display_name",
            ServerSortKey::Host => "host",
            ServerSortKey::HostName => "hostname",
            ServerSortKey::ImageRef => "image_ref",
            ServerSortKey::InstanceTypeId => "instance_type_id",
            ServerSortKey::KernelId => "kernel_id",
            ServerSortKey::KeyName => "key_name",
            ServerSortKey::LaunchIndex => "launch_index",
            ServerSortKey::LaunchedAt => "launched_at",
            ServerSortKey::LockedBy => "locked_by",
            ServerSortKey::Node => "node",
            ServerSortKey::PowerState => "power_state",
            ServerSortKey::Progress => "progress",
            ServerSortKey::ProjectId => "project_id",
            ServerSortKey::RamdiskId => "ramdisk_id",
            ServerSortKey::RootDeviceName => "root_device_name",
            ServerSortKey::TaskState => "task_state",
            ServerSortKey::TerminatedAt => "terminated_at",
            ServerSortKey::UpdatedAt => "updated_at",
            ServerSortKey::UserId => "user_id",
            ServerSortKey::Uuid => "uuid",
            ServerSortKey::VmState => "vm_state",
            _ => unreachable!()
        })
    }
}
