// Copyright 2018 Dmitry Tantsur <divius.inside@gmail.com>
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

//! JSON structures and protocol bits for the Network API.

#![allow(non_snake_case)]
#![allow(missing_docs)]

use std::net;

use chrono::{DateTime, FixedOffset};

use super::super::common;


protocol_enum! {
    #[doc = "IP protocol version."]
    enum IpVersion: u8 {
        V4 = 4,
        V6 = 6
    }
}

protocol_enum! {
    #[doc = "Possible network statuses."]
    enum NetworkStatus {
        Active = "ACTIVE",
        Down = "DOWN",
        Building = "BUILD",
        Error = "ERROR"
    }
}

protocol_enum! {
    #[doc = "Available sort keys."]
    enum NetworkSortKey {
        CreatedAt = "created_at",
        Id = "id",
        Name = "name",
        UpdatedAt = "updated_at"
    }
}

impl Default for NetworkSortKey {
    fn default() -> NetworkSortKey {
        NetworkSortKey::CreatedAt
    }
}

/// An network.
#[derive(Debug, Clone, Deserialize)]
pub struct Network {
    pub admin_state_up: bool,
    #[serde(default)]
    pub availability_zones: Vec<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub description: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub dns_domain: Option<String>,
    #[serde(rename = "router:external")]
    pub external: Option<bool>,
    pub id: String,
    #[serde(default)]
    pub is_default: Option<bool>,
    #[serde(default)]
    pub l2_adjacency: Option<bool>,
    #[serde(default)]
    pub mtu: Option<u32>,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub shared: bool,
    pub subnets: Vec<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

/// A network.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkRoot {
    pub network: Network
}

/// A list of networks.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworksRoot {
    pub networks: Vec<Network>
}

/// An extra DHCP option.
#[derive(Debug, Clone, Deserialize)]
pub struct PortExtraDhcpOption {
    #[serde(default)]
    pub ip_version: Option<IpVersion>,
    pub opt_name: String,
    pub opt_value: String,
}

/// A port's IP address.
#[derive(Debug, Clone, Deserialize)]
pub struct PortIpAddress {
    pub ip_address: net::IpAddr,
    pub subnet_id: String
}

/// A port.
#[derive(Debug, Clone, Deserialize)]
pub struct Port {
    pub admin_state_up: bool,
    #[serde(default)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub description: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub device_id: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub device_owner: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub dns_domain: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub dns_name: Option<String>,
    #[serde(default)]
    pub extra_dhcp_opts: Vec<PortExtraDhcpOption>,
    #[serde(default)]
    pub fixed_ips: Vec<PortIpAddress>,
    pub id: String,
    pub mac_address: String,
    #[serde(deserialize_with = "common::protocol::empty_as_none")]
    pub name: Option<String>,
    pub network_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub security_groups: Vec<String>,
    pub status: NetworkStatus,
    #[serde(default)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

protocol_enum! {
    #[doc = "Available sort keys."]
    enum PortSortKey {
        AdminStateUp = "admin_state_up",
        DeviceId = "device_id",
        DeviceOwner = "device_owner",
        Id = "id",
        MacAddress = "mac_address",
        Name = "name",
        NetworkId = "network_id",
        Status = "status"
    }
}

/// A port.
#[derive(Debug, Clone, Deserialize)]
pub struct PortRoot {
    pub port: Port
}

/// A list of ports.
#[derive(Debug, Clone, Deserialize)]
pub struct PortsRoot {
    pub ports: Vec<Port>
}
