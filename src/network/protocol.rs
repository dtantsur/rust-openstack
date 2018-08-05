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

use std::marker::PhantomData;
use std::net;

use chrono::{DateTime, FixedOffset};
use eui48::MacAddress;
use ipnet;

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

protocol_enum! {
    #[doc = "Available sort keys."]
    enum SubnetSortKey {
        Cidr = "cidr",
        DhcpEnabled = "enable_dhcp",
        GatewayIp = "gateway_ip",
        Id = "id",
        IpVersion = "ip_version",
        Ipv6AddressMode = "ipv6_address_mode",
        Ipv6RouterAdvertisementMode = "ipv6_ra_mode",
        Name = "name",
        NetworkId = "network_id"
    }
}

protocol_enum! {
    #[doc = "IPv6 modes for assigning IP addresses."]
    enum Ipv6Mode {
        DhcpStateful = "dhcpv6-stateful",
        DhcpStateless = "dhcpv6-stateless",
        Slaac = "slaac"
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortExtraDhcpOption {
    /// IP protocol version (if required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_version: Option<IpVersion>,
    /// Option name.
    #[serde(rename = "opt_name")]
    pub name: String,
    /// Option value.
    #[serde(rename = "opt_value")]
    pub value: String,
    #[doc(hidden)]
    #[serde(skip)]
    pub __nonexhaustive: PhantomData<()>,
}

impl PortExtraDhcpOption {
    /// Create a new DHCP option.
    pub fn new<S1, S2>(name: S1, value: S2) -> PortExtraDhcpOption
            where S1: Into<String>, S2: Into<String> {
        PortExtraDhcpOption {
            ip_version: None,
            name: name.into(),
            value: value.into(),
            __nonexhaustive: PhantomData,
        }
    }

    /// Create a new DHCP option with an IP version.
    pub fn new_with_ip_version<S1, S2>(name: S1, value: S2, ip_version: IpVersion)
            -> PortExtraDhcpOption where S1: Into<String>, S2: Into<String> {
        PortExtraDhcpOption {
            ip_version: Some(ip_version),
            name: name.into(),
            value: value.into(),
            __nonexhaustive: PhantomData,
        }
    }
}

/// A port's IP address.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedIp {
    #[serde(skip_serializing_if = "::std::net::IpAddr::is_unspecified")]
    pub ip_address: net::IpAddr,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub subnet_id: String
}

/// A port.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Port {
    pub admin_state_up: bool,
    #[serde(default, skip_serializing)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default,
            skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default,
            skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default,
            skip_serializing_if = "Option::is_none")]
    pub device_owner: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default,
            skip_serializing_if = "Option::is_none")]
    pub dns_domain: Option<String>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default,
            skip_serializing_if = "Option::is_none")]
    pub dns_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_dhcp_opts: Vec<PortExtraDhcpOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed_ips: Vec<FixedIp>,
    #[serde(skip_serializing)]
    pub id: String,
    #[serde(skip_serializing_if = "MacAddress::is_nil",
            serialize_with = "common::protocol::ser_mac")]
    pub mac_address: MacAddress,
    #[serde(deserialize_with = "common::protocol::empty_as_none",
            skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub network_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security_groups: Vec<String>,
    #[serde(skip_serializing)]
    pub status: NetworkStatus,
    #[serde(default, skip_serializing)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

/// A port.
#[derive(Debug, Clone, Serialize)]
pub struct PortUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_state_up: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_dhcp_opts: Option<Vec<PortExtraDhcpOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_ips: Option<Vec<FixedIp>>,
    #[serde(skip_serializing_if = "Option::is_none",
            serialize_with = "common::protocol::ser_opt_mac")]
    pub mac_address: Option<MacAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_groups: Option<Vec<String>>,
}

impl Default for PortUpdate {
    fn default() -> PortUpdate {
        PortUpdate {
            admin_state_up: None,
            description: None,
            device_id: None,
            device_owner: None,
            dns_domain: None,
            dns_name: None,
            extra_dhcp_opts: None,
            fixed_ips: None,
            mac_address: None,
            name: None,
            security_groups: None
        }
    }
}

/// A port.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortRoot {
    pub port: Port
}

/// A port update.
#[derive(Debug, Clone, Serialize)]
pub struct PortUpdateRoot {
    pub port: PortUpdate
}

/// A list of ports.
#[derive(Debug, Clone, Deserialize)]
pub struct PortsRoot {
    pub ports: Vec<Port>
}

/// An allocation pool.
#[derive(Copy, Debug, Clone, Deserialize)]
pub struct AllocationPool {
    /// Start IP address.
    pub start: net::IpAddr,
    /// End IP address.
    pub end: net::IpAddr
}

/// A host router.
#[derive(Copy, Debug, Clone, Deserialize)]
pub struct HostRoute {
    /// Destination network.
    pub destination: ipnet::IpNet,
    /// Next hop address.
    #[serde(rename = "nexthop")]
    pub next_hop: net::IpAddr,
}

/// A subnet.
#[derive(Debug, Clone, Deserialize)]
pub struct Subnet {
    #[serde(default)]
    pub allocation_pools: Vec<AllocationPool>,
    pub cidr: ipnet::IpNet,
    #[serde(default)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub description: Option<String>,
    #[serde(rename = "enable_dhcp")]
    pub dhcp_enabled: bool,
    #[serde(default)]
    pub dns_nameservers: Vec<String>,
    #[serde(default)]
    pub gateway_ip: Option<net::IpAddr>,
    #[serde(default)]
    pub host_routes: Vec<HostRoute>,
    pub id: String,
    pub ip_version: IpVersion,
    #[serde(default)]
    pub ipv6_address_mode: Option<Ipv6Mode>,
    #[serde(default, rename = "ipv6_ra_mode")]
    pub ipv6_router_advertisement_mode: Option<Ipv6Mode>,
    #[serde(deserialize_with = "common::protocol::empty_as_none")]
    pub name: Option<String>,
    pub network_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

/// A subnet.
#[derive(Debug, Clone, Deserialize)]
pub struct SubnetRoot {
    pub subnet: Subnet
}

/// A list of subnets.
#[derive(Debug, Clone, Deserialize)]
pub struct SubnetsRoot {
    pub subnets: Vec<Subnet>
}
