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

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use chrono::{DateTime, FixedOffset};
use hyper::Url;
use serde::{Deserialize, Deserializer};
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

/// All possible server statuses.
#[derive(Debug, Copy, Clone)]
pub enum ServerStatus {
    Active,
    Building,
    Deleted,
    Error,
    HardRebooting,
    Migrating,
    Paused,
    Rebooting,
    Resizing,
    RevertingResize,
    ShutOff,
    Suspended,
    Rescuing,
    Shelved,
    ShelvedOffloaded,
    SoftDeleted,
    Unknown,
    UpdatingPassword,
    VerifyingResize,
    #[doc(hidden)]
    __Nonexhaustive,
}

/// Address of a server.
#[derive(Clone, Copy, Debug)]
pub enum AddressType {
    Fixed,
    Floating,
    Unknown
}

/// Address of a server.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerAddress {
    /// IP (v4 of v6) address.
    pub addr: IpAddr,
    /// MAC address (if available).
    #[serde(rename = "OS-EXT-IPS-MAC:mac_addr")]
    pub mac_addr: Option<String>,
    /// Address type (if known).
    #[serde(rename = "OS-EXT-IPS:type", default,
            deserialize_with = "de_address_type")]
    pub addr_type: AddressType
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ref {
    pub id: String,
    pub links: Vec<Link>
}

#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    #[serde(deserialize_with = "utils::empty_as_none")]
    pub accessIPv4: Option<Ipv4Addr>,
    #[serde(deserialize_with = "utils::empty_as_none")]
    pub accessIPv6: Option<Ipv6Addr>,
    #[serde(default)]
    pub addresses: HashMap<String, Vec<ServerAddress>>,
    #[serde(rename = "OS-EXT-AZ:availability_zone")]
    pub availability_zone: String,
    pub created: DateTime<FixedOffset>,
    pub flavor: Ref,
    pub id: String,
    pub image: Ref,
    pub name: String,
    #[serde(deserialize_with = "de_server_status", default)]
    pub status: ServerStatus,
    pub tenant_id: String,
    pub updated: DateTime<FixedOffset>,
    pub user_id: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct ServersRoot {
    pub servers: Vec<ServerSummary>
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerRoot {
    pub server: Server
}

#[derive(Clone, Debug, Deserialize)]
pub struct Link {
    pub href: String,
    pub rel: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub links: Vec<Link>,
    pub status: String,
    pub version: String,
    pub min_version: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionsRoot {
    pub versions: Vec<Version>
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionRoot {
    pub version: Version
}


impl Version {
    pub fn to_service_info(&self) -> ApiResult<ServiceInfo> {
        let current_version = if self.version.is_empty() {
            None
        } else {
            Some(ApiVersion::from_str(&self.version)?)
        };

        let minimum_version = if self.min_version.is_empty() {
            None
        } else {
            Some(ApiVersion::from_str(&self.min_version)?)
        };

        let endpoint = match self.links.iter().find(|x| &x.rel == "self") {
            Some(link) => Url::parse(&link.href)?,
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

impl Default for ServerStatus {
    fn default() -> ServerStatus {
        ServerStatus::Unknown
    }
}

impl Default for AddressType {
    fn default() -> AddressType {
        AddressType::Unknown
    }
}

fn de_address_type<D>(des: D) -> Result<AddressType, D::Error>
        where D: Deserializer {
    Ok(match String::deserialize(des)?.as_ref() {
        "fixed" => AddressType::Fixed,
        "floating" => AddressType::Floating,
        _ => Default::default()
    })
}

fn de_server_status<D>(des: D) -> Result<ServerStatus, D::Error>
        where D: Deserializer {
    let s = String::deserialize(des)?;
    Ok(match s.as_ref() {
        "ACTIVE" => ServerStatus::Active,
        "BUILD" => ServerStatus::Building,
        "DELETED" => ServerStatus::Deleted,
        "ERROR" => ServerStatus::Error,
        "HARD_REBOOT" => ServerStatus::HardRebooting,
        "MIGRATING" => ServerStatus::Migrating,
        "PAUSED" => ServerStatus::Paused,
        "REBOOT" => ServerStatus::Rebooting,
        "RESIZE" => ServerStatus::Resizing,
        "REVERT_RESIZE" => ServerStatus::RevertingResize,
        "SHUTOFF" => ServerStatus::ShutOff,
        "SUSPENDED" => ServerStatus::Suspended,
        "RESCUE" => ServerStatus::Rescuing,
        "SHELVED" => ServerStatus::Shelved,
        "SHELVED_OFFLOADED" => ServerStatus::ShelvedOffloaded,
        "SOFT_DELETED" => ServerStatus::SoftDeleted,
        "PASSWORD" => ServerStatus::UpdatingPassword,
        "VERIFY_RESIZE" => ServerStatus::VerifyingResize,
        _ => {
            warn!("Got unknown server status {}", s);
            Default::default()
        }
    })
}
