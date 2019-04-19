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

//! Foundation bits exposing the Network API.

use std::fmt::Debug;
use std::sync::Arc;

use serde::Serialize;

use super::super::common::ApiVersion;
use super::super::session::{RequestBuilderExt, ServiceType, Session};
use super::super::utils::{self, ResultExt};
use super::super::Result;
use super::protocol;

/// Service type of Network API NetworkService.
#[derive(Copy, Clone, Debug)]
pub struct NetworkService;

impl ServiceType for NetworkService {
    fn catalog_type() -> &'static str {
        "network"
    }

    fn major_version_supported(version: ApiVersion) -> bool {
        version == ApiVersion(2, 0)
    }
}

/// Create a floating IP.
pub fn create_floating_ip(
    session: &Arc<Session>,
    request: protocol::FloatingIp,
) -> Result<protocol::FloatingIp> {
    debug!("Creating a new floating IP with {:?}", request);
    let body = protocol::FloatingIpRoot {
        floatingip: request,
    };
    let floating_ip = session
        .post::<NetworkService>(&["floatingips"], None)?
        .json(&body)
        .receive_json::<protocol::FloatingIpRoot>()?
        .floatingip;
    debug!("Created floating IP {:?}", floating_ip);
    Ok(floating_ip)
}

/// Create a network.
pub fn create_network(
    session: &Arc<Session>,
    request: protocol::Network,
) -> Result<protocol::Network> {
    debug!("Creating a new network with {:?}", request);
    let body = protocol::NetworkRoot { network: request };
    let network = session
        .post::<NetworkService>(&["networks"], None)?
        .json(&body)
        .receive_json::<protocol::NetworkRoot>()?
        .network;
    debug!("Created network {:?}", network);
    Ok(network)
}

/// Create a port.
pub fn create_port(session: &Arc<Session>, request: protocol::Port) -> Result<protocol::Port> {
    debug!("Creating a new port with {:?}", request);
    let body = protocol::PortRoot { port: request };
    let port = session
        .post::<NetworkService>(&["ports"], None)?
        .json(&body)
        .receive_json::<protocol::PortRoot>()?
        .port;
    debug!("Created port {:?}", port);
    Ok(port)
}

/// Create a subnet.
pub fn create_subnet(
    session: &Arc<Session>,
    request: protocol::Subnet,
) -> Result<protocol::Subnet> {
    debug!("Creating a new subnet with {:?}", request);
    let body = protocol::SubnetRoot { subnet: request };
    let subnet = session
        .post::<NetworkService>(&["subnets"], None)?
        .json(&body)
        .receive_json::<protocol::SubnetRoot>()?
        .subnet;
    debug!("Created subnet {:?}", subnet);
    Ok(subnet)
}

/// Delete a floating IP.
pub fn delete_floating_ip<S: AsRef<str>>(session: &Arc<Session>, id: S) -> Result<()> {
    debug!("Deleting floating IP {}", id.as_ref());
    session
        .delete::<NetworkService>(&["floatingips", id.as_ref()], None)?
        .commit()?;
    debug!("Floating IP {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a network.
pub fn delete_network<S: AsRef<str>>(session: &Arc<Session>, id: S) -> Result<()> {
    debug!("Deleting network {}", id.as_ref());
    session
        .delete::<NetworkService>(&["networks", id.as_ref()], None)?
        .commit()?;
    debug!("Network {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a port.
pub fn delete_port<S: AsRef<str>>(session: &Arc<Session>, id: S) -> Result<()> {
    debug!("Deleting port {}", id.as_ref());
    session
        .delete::<NetworkService>(&["ports", id.as_ref()], None)?
        .commit()?;
    debug!("Port {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a subnet.
pub fn delete_subnet<S: AsRef<str>>(session: &Arc<Session>, id: S) -> Result<()> {
    debug!("Deleting subnet {}", id.as_ref());
    session
        .delete::<NetworkService>(&["subnets", id.as_ref()], None)?
        .commit()?;
    debug!("Subnet {} was deleted", id.as_ref());
    Ok(())
}

/// Get a floating IP.
pub fn get_floating_ip<S: AsRef<str>>(
    session: &Arc<Session>,
    id: S,
) -> Result<protocol::FloatingIp> {
    trace!("Get floating IP by ID {}", id.as_ref());
    let floatingip = session
        .get_json::<NetworkService, protocol::FloatingIpRoot>(&["floatingips", id.as_ref()], None)?
        .floatingip;
    trace!("Received {:?}", floatingip);
    Ok(floatingip)
}

/// Get a network.
pub fn get_network<S: AsRef<str>>(
    session: &Arc<Session>,
    id_or_name: S,
) -> Result<protocol::Network> {
    let s = id_or_name.as_ref();
    get_network_by_id(session, s).if_not_found_then(|| get_network_by_name(session, s))
}

/// Get a network by its ID.
pub fn get_network_by_id<S: AsRef<str>>(
    session: &Arc<Session>,
    id: S,
) -> Result<protocol::Network> {
    trace!("Get network by ID {}", id.as_ref());
    let network = session
        .get_json::<NetworkService, protocol::NetworkRoot>(&["networks", id.as_ref()], None)?
        .network;
    trace!("Received {:?}", network);
    Ok(network)
}

/// Get a network by its name.
pub fn get_network_by_name<S: AsRef<str>>(
    session: &Arc<Session>,
    name: S,
) -> Result<protocol::Network> {
    trace!("Get network by name {}", name.as_ref());
    let items = session
        .get_json_query::<NetworkService, _, protocol::NetworksRoot>(
            &["networks"],
            &[("name", name.as_ref())],
            None,
        )?
        .networks;
    let result = utils::one(
        items,
        "Network with given name or ID not found",
        "Too many networks found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Get a port.
pub fn get_port<S: AsRef<str>>(session: &Arc<Session>, id_or_name: S) -> Result<protocol::Port> {
    let s = id_or_name.as_ref();
    get_port_by_id(session, s).if_not_found_then(|| get_port_by_name(session, s))
}

/// Get a port by its ID.
pub fn get_port_by_id<S: AsRef<str>>(session: &Arc<Session>, id: S) -> Result<protocol::Port> {
    trace!("Get port by ID {}", id.as_ref());
    let port = session
        .get_json::<NetworkService, protocol::PortRoot>(&["ports", id.as_ref()], None)?
        .port;
    trace!("Received {:?}", port);
    Ok(port)
}

/// Get a port by its name.
pub fn get_port_by_name<S: AsRef<str>>(session: &Arc<Session>, name: S) -> Result<protocol::Port> {
    trace!("Get port by name {}", name.as_ref());
    let items = session
        .get_json_query::<NetworkService, _, protocol::PortsRoot>(
            &["ports"],
            &[("name", name.as_ref())],
            None,
        )?
        .ports;
    let result = utils::one(
        items,
        "Port with given name or ID not found",
        "Too many ports found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Get a subnet.
pub fn get_subnet<S: AsRef<str>>(
    session: &Arc<Session>,
    id_or_name: S,
) -> Result<protocol::Subnet> {
    let s = id_or_name.as_ref();
    get_subnet_by_id(session, s).if_not_found_then(|| get_subnet_by_name(session, s))
}

/// Get a subnet by its ID.
pub fn get_subnet_by_id<S: AsRef<str>>(session: &Arc<Session>, id: S) -> Result<protocol::Subnet> {
    trace!("Get subnet by ID {}", id.as_ref());
    let subnet = session
        .get_json::<NetworkService, protocol::SubnetRoot>(&["subnets", id.as_ref()], None)?
        .subnet;
    trace!("Received {:?}", subnet);
    Ok(subnet)
}

/// Get a subnet by its name.
pub fn get_subnet_by_name<S: AsRef<str>>(
    session: &Arc<Session>,
    name: S,
) -> Result<protocol::Subnet> {
    trace!("Get subnet by name {}", name.as_ref());
    let items = session
        .get_json_query::<NetworkService, _, protocol::SubnetsRoot>(
            &["subnets"],
            &[("name", name.as_ref())],
            None,
        )?
        .subnets;
    let result = utils::one(
        items,
        "Subnet with given name or ID not found",
        "Too many subnets found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// List floating IPs.
pub fn list_floating_ips<Q: Serialize + Debug>(
    session: &Arc<Session>,
    query: &Q,
) -> Result<Vec<protocol::FloatingIp>> {
    trace!("Listing floating IPs with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, protocol::FloatingIpsRoot>(
            &["floatingips"],
            query,
            None,
        )?
        .floatingips;
    trace!("Received floating IPs: {:?}", result);
    Ok(result)
}

/// List networks.
pub fn list_networks<Q: Serialize + Debug>(
    session: &Arc<Session>,
    query: &Q,
) -> Result<Vec<protocol::Network>> {
    trace!("Listing networks with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, protocol::NetworksRoot>(&["networks"], query, None)?
        .networks;
    trace!("Received networks: {:?}", result);
    Ok(result)
}

/// List ports.
pub fn list_ports<Q: Serialize + Debug>(
    session: &Arc<Session>,
    query: &Q,
) -> Result<Vec<protocol::Port>> {
    trace!("Listing ports with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, protocol::PortsRoot>(&["ports"], query, None)?
        .ports;
    trace!("Received ports: {:?}", result);
    Ok(result)
}

/// List subnets.
pub fn list_subnets<Q: Serialize + Debug>(
    session: &Arc<Session>,
    query: &Q,
) -> Result<Vec<protocol::Subnet>> {
    trace!("Listing subnets with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, protocol::SubnetsRoot>(&["subnets"], query, None)?
        .subnets;
    trace!("Received subnets: {:?}", result);
    Ok(result)
}

/// Update a floating IP.
pub fn update_floating_ip<S: AsRef<str>>(
    session: &Arc<Session>,
    id: S,
    update: protocol::FloatingIpUpdate,
) -> Result<protocol::FloatingIp> {
    debug!("Updating floatingIP {} with {:?}", id.as_ref(), update);
    let body = protocol::FloatingIpUpdateRoot { floatingip: update };
    let floating_ip = session
        .put::<NetworkService>(&["floatingips", id.as_ref()], None)?
        .json(&body)
        .receive_json::<protocol::FloatingIpRoot>()?
        .floatingip;
    debug!("Updated floating IP {:?}", floating_ip);
    Ok(floating_ip)
}

/// Update a network.
pub fn update_network<S: AsRef<str>>(
    session: &Arc<Session>,
    id: S,
    update: protocol::NetworkUpdate,
) -> Result<protocol::Network> {
    debug!("Updating network {} with {:?}", id.as_ref(), update);
    let body = protocol::NetworkUpdateRoot { network: update };
    let network = session
        .put::<NetworkService>(&["networks", id.as_ref()], None)?
        .json(&body)
        .receive_json::<protocol::NetworkRoot>()?
        .network;
    debug!("Updated network {:?}", network);
    Ok(network)
}

/// Update a port.
pub fn update_port<S: AsRef<str>>(
    session: &Arc<Session>,
    id: S,
    update: protocol::PortUpdate,
) -> Result<protocol::Port> {
    debug!("Updating port {} with {:?}", id.as_ref(), update);
    let body = protocol::PortUpdateRoot { port: update };
    let port = session
        .put::<NetworkService>(&["ports", id.as_ref()], None)?
        .json(&body)
        .receive_json::<protocol::PortRoot>()?
        .port;
    debug!("Updated port {:?}", port);
    Ok(port)
}

/// Update a subnet.
pub fn update_subnet<S: AsRef<str>>(
    session: &Arc<Session>,
    id: S,
    update: protocol::SubnetUpdate,
) -> Result<protocol::Subnet> {
    debug!("Updating subnet {} with {:?}", id.as_ref(), update);
    let body = protocol::SubnetUpdateRoot { subnet: update };
    let subnet = session
        .put::<NetworkService>(&["subnets", id.as_ref()], None)?
        .json(&body)
        .receive_json::<protocol::SubnetRoot>()?
        .subnet;
    debug!("Updated subnet {:?}", subnet);
    Ok(subnet)
}
