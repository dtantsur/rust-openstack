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
use std::ops::Not;

use chrono::{DateTime, FixedOffset};
use eui48::MacAddress;
use ipnet;
use osproto::common::empty_as_default;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

protocol_enum! {
    #[doc = "Possible floating IP statuses."]
    enum FloatingIpStatus {
        Active = "ACTIVE",
        Down = "DOWN",
        Error = "ERROR"
    }
}

protocol_enum! {
    #[doc = "Available sort keys."]
    enum FloatingIpSortKey {
        FixedIpAddress = "fixed_ip_address",
        FloatingIpAddress = "floating_ip_address",
        FloatingNetworkId = "floating_network_id",
        Id = "id",
        RouterId = "router_id",
        Status = "status"
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Network {
    pub admin_state_up: bool,
    #[serde(default, skip_serializing)]
    pub availability_zones: Vec<String>,
    #[serde(default, skip_serializing)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dns_domain: Option<String>,
    #[serde(rename = "router:external", skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(default, skip_serializing)]
    pub l2_adjacency: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,
    #[serde(
        deserialize_with = "empty_as_default",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_security_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub shared: bool,
    #[serde(skip_serializing)]
    pub status: NetworkStatus,
    #[serde(skip_serializing)]
    pub subnets: Vec<String>,
    #[serde(default, skip_serializing)]
    pub updated_at: Option<DateTime<FixedOffset>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlan_transparent: Option<bool>,
}

impl Default for Network {
    fn default() -> Network {
        Network {
            admin_state_up: true,
            availability_zones: Vec::new(),
            created_at: None,
            description: None,
            dns_domain: None,
            external: None,
            id: String::new(),
            is_default: None,
            l2_adjacency: None,
            mtu: None,
            name: None,
            port_security_enabled: None,
            project_id: None,
            shared: false,
            status: NetworkStatus::Active,
            subnets: Vec::new(),
            updated_at: None,
            vlan_transparent: None,
        }
    }
}

/// A network.
#[derive(Debug, Clone, Default, Serialize)]
pub struct NetworkUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_state_up: Option<bool>,
    #[serde(rename = "router:external", skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_security_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,
}

/// A network.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkRoot {
    pub network: Network,
}

/// A network.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkUpdateRoot {
    pub network: NetworkUpdate,
}

/// A list of networks.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworksRoot {
    pub networks: Vec<Network>,
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
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        PortExtraDhcpOption {
            ip_version: None,
            name: name.into(),
            value: value.into(),
            __nonexhaustive: PhantomData,
        }
    }

    /// Create a new DHCP option with an IP version.
    pub fn new_with_ip_version<S1, S2>(
        name: S1,
        value: S2,
        ip_version: IpVersion,
    ) -> PortExtraDhcpOption
    where
        S1: Into<String>,
        S2: Into<String>,
    {
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
    pub subnet_id: String,
}

/// A port.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Port {
    pub admin_state_up: bool,
    #[serde(default, skip_serializing)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub device_id: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub device_owner: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dns_domain: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dns_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_dhcp_opts: Vec<PortExtraDhcpOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed_ips: Vec<FixedIp>,
    #[serde(skip_serializing)]
    pub id: String,
    #[serde(
        skip_serializing_if = "MacAddress::is_nil",
        serialize_with = "common::protocol::ser_mac"
    )]
    pub mac_address: MacAddress,
    #[serde(
        deserialize_with = "empty_as_default",
        skip_serializing_if = "Option::is_none"
    )]
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
#[derive(Debug, Clone, Serialize, Default)]
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "common::protocol::ser_opt_mac"
    )]
    pub mac_address: Option<MacAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_groups: Option<Vec<String>>,
}

/// A port.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortRoot {
    pub port: Port,
}

/// A port update.
#[derive(Debug, Clone, Serialize)]
pub struct PortUpdateRoot {
    pub port: PortUpdate,
}

/// A list of ports.
#[derive(Debug, Clone, Deserialize)]
pub struct PortsRoot {
    pub ports: Vec<Port>,
}

/// An allocation pool.
#[derive(Copy, Debug, Clone, Deserialize, Serialize)]
pub struct AllocationPool {
    /// Start IP address.
    pub start: net::IpAddr,
    /// End IP address.
    pub end: net::IpAddr,
}

/// A host router.
#[derive(Copy, Debug, Clone, Deserialize, Serialize)]
pub struct HostRoute {
    /// Destination network.
    pub destination: ipnet::IpNet,
    /// Next hop address.
    #[serde(rename = "nexthop")]
    pub next_hop: net::IpAddr,
}

/// A subnet.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subnet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allocation_pools: Vec<AllocationPool>,
    pub cidr: ipnet::IpNet,
    #[serde(default, skip_serializing)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(rename = "enable_dhcp")]
    pub dhcp_enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dns_nameservers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway_ip: Option<net::IpAddr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_routes: Vec<HostRoute>,
    #[serde(skip_serializing)]
    pub id: String,
    pub ip_version: IpVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipv6_address_mode: Option<Ipv6Mode>,
    #[serde(
        default,
        rename = "ipv6_ra_mode",
        skip_serializing_if = "Option::is_none"
    )]
    pub ipv6_router_advertisement_mode: Option<Ipv6Mode>,
    #[serde(
        deserialize_with = "empty_as_default",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<String>,
    pub network_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

impl Subnet {
    pub(crate) fn empty(cidr: ipnet::IpNet) -> Subnet {
        Subnet {
            allocation_pools: Vec::new(),
            cidr,
            created_at: None,
            description: None,
            dhcp_enabled: true,
            dns_nameservers: Vec::new(),
            gateway_ip: None,
            host_routes: Vec::new(),
            id: String::new(),
            ip_version: match cidr {
                ipnet::IpNet::V4(..) => IpVersion::V4,
                ipnet::IpNet::V6(..) => IpVersion::V6,
            },
            ipv6_address_mode: None,
            ipv6_router_advertisement_mode: None,
            name: None,
            network_id: String::new(),
            project_id: None,
            updated_at: None,
        }
    }
}

/// A subnet.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SubnetUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_pools: Option<Vec<AllocationPool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enable_dhcp", skip_serializing_if = "Option::is_none")]
    pub dhcp_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_nameservers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_ip: Option<net::IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_routes: Option<Vec<HostRoute>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A subnet.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubnetRoot {
    pub subnet: Subnet,
}

/// A subnet.
#[derive(Debug, Clone, Serialize)]
pub struct SubnetUpdateRoot {
    pub subnet: SubnetUpdate,
}

/// A list of subnets.
#[derive(Debug, Clone, Deserialize)]
pub struct SubnetsRoot {
    pub subnets: Vec<Subnet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PortForwarding {
    /// TCP or UDP port used by floating IP.
    pub external_port: u16,
    /// Fixed IP address of internal port.
    pub internal_ip_address: net::IpAddr,
    /// TCP or UDP port used by internal port.
    pub internal_port: u16,
    /// Network IP protocol.
    pub protocol: String,
}

/// A floating IP.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FloatingIp {
    #[serde(default, skip_serializing)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dns_domain: Option<String>,
    #[serde(
        deserialize_with = "empty_as_default",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dns_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_ip_address: Option<net::IpAddr>,
    #[serde(skip_serializing_if = "::std::net::IpAddr::is_unspecified")]
    pub floating_ip_address: net::IpAddr,
    pub floating_network_id: String,
    #[serde(skip_serializing)]
    pub id: String,
    #[serde(default)]
    pub port_id: Option<String>,
    #[serde(default, skip_serializing)]
    pub port_forwardings: Vec<PortForwarding>,
    #[serde(default, skip_serializing)]
    pub router_id: Option<String>,
    #[serde(skip_serializing)]
    pub status: FloatingIpStatus,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<String>,
    #[serde(default, skip_serializing)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

/// A port.
#[derive(Debug, Clone, Serialize, Default)]
pub struct FloatingIpUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_ip_address: Option<net::IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_id: Option<Value>,
}

/// A floating IP.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FloatingIpRoot {
    pub floatingip: FloatingIp,
}

/// A floating IP.
#[derive(Debug, Clone, Serialize)]
pub struct FloatingIpUpdateRoot {
    pub floatingip: FloatingIpUpdate,
}

/// Floating IPs.
#[derive(Debug, Clone, Deserialize)]
pub struct FloatingIpsRoot {
    pub floatingips: Vec<FloatingIp>,
}
