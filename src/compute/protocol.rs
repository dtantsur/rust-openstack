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

use chrono::{DateTime, FixedOffset};

use super::super::common;

protocol_enum! {
    #[doc = "Available sort keys."]
    enum ServerSortKey {
        AccessIpv4 = "access_ip_v4",
        AccessIpv6 = "access_ip_v6",
        AutoDiskConfig = "auto_disk_config",
        AvailabilityZone = "availability_zone",
        ConfigDrive = "config_drive",
        CreatedAt = "created_at",
        DisplayDescription = "display_description",
        DisplayName = "display_name",
        Host = "host",
        HostName = "hostname",
        ImageRef = "image_ref",
        InstanceTypeId = "instance_type_id",
        KernelId = "kernel_id",
        KeyName = "key_name",
        LaunchIndex = "launch_index",
        LaunchedAt = "launched_at",
        LockedBy = "locked_by",
        Node = "node",
        PowerState = "power_state",
        Progress = "progress",
        ProjectId = "project_id",
        RamdiskId = "ramdisk_id",
        RootDeviceName = "root_device_name",
        TaskState = "task_state",
        TerminatedAt = "terminated_at",
        UpdatedAt = "updated_at",
        UserId = "user_id",
        Uuid = "uuid",
        VmState = "vm_state"
    }
}

protocol_enum! {
    #[doc = "Possible server statuses."]
    enum ServerStatus {
        Active = "ACTIVE",
        Building = "BUILD",
        Deleted = "DELETED",
        Error = "ERROR",
        HardRebooting = "HARD_REBOOT",
        Migrating = "MIGRATING",
        Paused = "PAUSED",
        Rebooting = "REBOOT",
        Resizing = "RESIZE",
        RevertingResize = "REVERT_RESIZE",
        ShutOff = "SHUTOFF",
        Suspended = "SUSPENDED",
        Rescuing = "RESCUE",
        Shelved = "SHELVED",
        ShelvedOffloaded = "SHELVED_OFFLOADED",
        SoftDeleted = "SOFT_DELETED",
        Unknown = "UNKNOWN",
        UpdatingPassword = "PASSWORD",
        VerifyingResize = "VERIFY_RESIZE"
    }
}

protocol_enum! {
    #[doc = "Possible power states."]
    enum ServerPowerState: u8 {
        NoState = 0,
        Running = 1,
        Paused = 3,
        Shutdown = 4,
        Crashed = 6,
        Suspended = 7
    }
}

protocol_enum! {
    #[doc = "Reboot type."]
    enum RebootType {
        Hard = "HARD",
        Soft = "SOFT"
    }
}

protocol_enum! {
    #[doc = "Type of a server address."]
    enum AddressType {
        Fixed = "fixed",
        Floating = "floating"
    }
}

protocol_enum! {
    #[doc = "Type of a key pair."]
    enum KeyPairType {
        SSH = "ssh",
        X509 = "x509"
    }
}

/// Address of a server.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerAddress {
    /// IP (v4 of v6) address.
    pub addr: IpAddr,
    /// MAC address (if available).
    #[serde(rename = "OS-EXT-IPS-MAC:mac_addr", default)]
    pub mac_addr: Option<String>,
    /// Address type (if known).
    #[serde(rename = "OS-EXT-IPS:type", default)]
    pub addr_type: Option<AddressType>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExtraSpecsRoot {
    pub extra_specs: HashMap<String, String>,
}

/// A summary information of a flavor used for a server.
#[derive(Clone, Debug)]
pub struct ServerFlavor {
    /// Ephemeral disk size in GiB.
    pub ephemeral_size: u64,
    /// Extra specs (if present).
    pub extra_specs: Option<HashMap<String, String>>,
    /// Name of the original flavor.
    pub original_name: String,
    /// RAM size in MiB.
    pub ram_size: u64,
    /// Root disk size in GiB.
    pub root_size: u64,
    /// Swap disk size in MiB.
    pub swap_size: u64,
    /// VCPU count.
    pub vcpu_count: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    #[serde(
        deserialize_with = "common::protocol::empty_as_none",
        default,
        rename = "accessIPv4"
    )]
    pub access_ipv4: Option<Ipv4Addr>,
    #[serde(
        deserialize_with = "common::protocol::empty_as_none",
        default,
        rename = "accessIPv6"
    )]
    pub access_ipv6: Option<Ipv6Addr>,
    #[serde(default)]
    pub addresses: HashMap<String, Vec<ServerAddress>>,
    #[serde(rename = "OS-EXT-AZ:availability_zone")]
    pub availability_zone: String,
    #[serde(rename = "created")]
    pub created_at: DateTime<FixedOffset>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub description: Option<String>,
    // TODO(dtantsur): flavor in newer versions
    pub flavor: common::protocol::Ref,
    #[serde(
        deserialize_with = "common::protocol::empty_as_default",
        rename = "config_drive"
    )]
    pub has_config_drive: bool,
    pub id: String,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub image: Option<common::protocol::Ref>,
    #[serde(
        rename = "key_name",
        deserialize_with = "common::protocol::empty_as_none",
        default
    )]
    pub key_pair_name: Option<String>,
    pub name: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub status: ServerStatus,
    #[serde(rename = "OS-EXT-STS:power_state", default)]
    pub power_state: ServerPowerState,
    pub tenant_id: String,
    #[serde(rename = "updated")]
    pub updated_at: DateTime<FixedOffset>,
    pub user_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServersRoot {
    pub servers: Vec<common::protocol::IdAndName>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServersDetailRoot {
    pub servers: Vec<Server>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerRoot {
    pub server: Server,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ServerNetwork {
    Network { uuid: String },
    Port { port: String },
    FixedIp { fixed_ip: Ipv4Addr },
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerCreate {
    pub flavorRef: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imageRef: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_name: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    pub name: String,
    pub networks: Vec<ServerNetwork>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerCreateRoot {
    pub server: ServerCreate,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreatedServerRoot {
    pub server: common::protocol::Ref,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Flavor {
    #[serde(rename = "OS-FLV-EXT-DATA:ephemeral", default)]
    pub ephemeral: u64,
    #[serde(default, deserialize_with = "common::protocol::empty_as_none")]
    pub description: Option<String>,
    pub disk: u64,
    #[serde(default)]
    pub extra_specs: Option<HashMap<String, String>>,
    pub id: String,
    #[serde(
        rename = "os-flavor-access:is_public",
        default = "default_flavor_is_public"
    )]
    pub is_public: bool,
    pub name: String,
    pub ram: u64,
    pub rxtx_factor: f32,
    #[serde(deserialize_with = "common::protocol::empty_as_default")]
    pub swap: u64,
    pub vcpus: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FlavorsRoot {
    pub flavors: Vec<common::protocol::IdAndName>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FlavorsDetailRoot {
    pub flavors: Vec<Flavor>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FlavorRoot {
    pub flavor: Flavor,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyPair {
    pub fingerprint: String,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub key_type: Option<KeyPairType>,
    pub name: String,
    pub public_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct KeyPairCreate {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub key_type: Option<KeyPairType>,
    pub name: String,
    pub public_key: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyPairRoot {
    pub keypair: KeyPair,
}

#[derive(Clone, Debug, Serialize)]
pub struct KeyPairCreateRoot {
    pub keypair: KeyPairCreate,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyPairsRoot {
    pub keypairs: Vec<KeyPairRoot>,
}

impl Default for ServerStatus {
    fn default() -> ServerStatus {
        ServerStatus::Unknown
    }
}

impl Default for ServerPowerState {
    fn default() -> ServerPowerState {
        ServerPowerState::NoState
    }
}

#[inline]
fn default_flavor_is_public() -> bool {
    true
}
