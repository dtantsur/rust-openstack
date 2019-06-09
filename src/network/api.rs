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

use serde::Serialize;

use super::super::common::ApiVersion;
use super::super::session::{ServiceType, Session};
use super::super::utils::{self, ResultExt};
use super::super::Result;
use super::protocol::*;

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
pub fn create_floating_ip(session: &Session, request: FloatingIp) -> Result<FloatingIp> {
    debug!("Creating a new floating IP with {:?}", request);
    let body = FloatingIpRoot {
        floatingip: request,
    };
    let floating_ip = session
        .post_json::<NetworkService, _, FloatingIpRoot>(&["floatingips"], body, None)?
        .floatingip;
    debug!("Created floating IP {:?}", floating_ip);
    Ok(floating_ip)
}

/// Create a network.
pub fn create_network(session: &Session, request: Network) -> Result<Network> {
    debug!("Creating a new network with {:?}", request);
    let body = NetworkRoot { network: request };
    let network = session
        .post_json::<NetworkService, _, NetworkRoot>(&["networks"], body, None)?
        .network;
    debug!("Created network {:?}", network);
    Ok(network)
}

/// Create a port.
pub fn create_port(session: &Session, request: Port) -> Result<Port> {
    debug!("Creating a new port with {:?}", request);
    let body = PortRoot { port: request };
    let port = session
        .post_json::<NetworkService, _, PortRoot>(&["ports"], body, None)?
        .port;
    debug!("Created port {:?}", port);
    Ok(port)
}

/// Create a subnet.
pub fn create_subnet(session: &Session, request: Subnet) -> Result<Subnet> {
    debug!("Creating a new subnet with {:?}", request);
    let body = SubnetRoot { subnet: request };
    let subnet = session
        .post_json::<NetworkService, _, SubnetRoot>(&["subnets"], body, None)?
        .subnet;
    debug!("Created subnet {:?}", subnet);
    Ok(subnet)
}

/// Delete a floating IP.
pub fn delete_floating_ip<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting floating IP {}", id.as_ref());
    let _ = session.delete::<NetworkService>(&["floatingips", id.as_ref()], None)?;
    debug!("Floating IP {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a network.
pub fn delete_network<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting network {}", id.as_ref());
    let _ = session.delete::<NetworkService>(&["networks", id.as_ref()], None)?;
    debug!("Network {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a port.
pub fn delete_port<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting port {}", id.as_ref());
    let _ = session.delete::<NetworkService>(&["ports", id.as_ref()], None)?;
    debug!("Port {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a subnet.
pub fn delete_subnet<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting subnet {}", id.as_ref());
    let _ = session.delete::<NetworkService>(&["subnets", id.as_ref()], None)?;
    debug!("Subnet {} was deleted", id.as_ref());
    Ok(())
}

/// Get a floating IP.
pub fn get_floating_ip<S: AsRef<str>>(session: &Session, id: S) -> Result<FloatingIp> {
    trace!("Get floating IP by ID {}", id.as_ref());
    let floatingip = session
        .get_json::<NetworkService, FloatingIpRoot>(&["floatingips", id.as_ref()], None)?
        .floatingip;
    trace!("Received {:?}", floatingip);
    Ok(floatingip)
}

/// Get a network.
pub fn get_network<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Network> {
    let s = id_or_name.as_ref();
    get_network_by_id(session, s).if_not_found_then(|| get_network_by_name(session, s))
}

/// Get a network by its ID.
pub fn get_network_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Network> {
    trace!("Get network by ID {}", id.as_ref());
    let network = session
        .get_json::<NetworkService, NetworkRoot>(&["networks", id.as_ref()], None)?
        .network;
    trace!("Received {:?}", network);
    Ok(network)
}

/// Get a network by its name.
pub fn get_network_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Network> {
    trace!("Get network by name {}", name.as_ref());
    let items = session
        .get_json_query::<NetworkService, _, NetworksRoot>(
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
pub fn get_port<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Port> {
    let s = id_or_name.as_ref();
    get_port_by_id(session, s).if_not_found_then(|| get_port_by_name(session, s))
}

/// Get a port by its ID.
pub fn get_port_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Port> {
    trace!("Get port by ID {}", id.as_ref());
    let port = session
        .get_json::<NetworkService, PortRoot>(&["ports", id.as_ref()], None)?
        .port;
    trace!("Received {:?}", port);
    Ok(port)
}

/// Get a port by its name.
pub fn get_port_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Port> {
    trace!("Get port by name {}", name.as_ref());
    let items = session
        .get_json_query::<NetworkService, _, PortsRoot>(
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
pub fn get_subnet<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Subnet> {
    let s = id_or_name.as_ref();
    get_subnet_by_id(session, s).if_not_found_then(|| get_subnet_by_name(session, s))
}

/// Get a subnet by its ID.
pub fn get_subnet_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Subnet> {
    trace!("Get subnet by ID {}", id.as_ref());
    let subnet = session
        .get_json::<NetworkService, SubnetRoot>(&["subnets", id.as_ref()], None)?
        .subnet;
    trace!("Received {:?}", subnet);
    Ok(subnet)
}

/// Get a subnet by its name.
pub fn get_subnet_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Subnet> {
    trace!("Get subnet by name {}", name.as_ref());
    let items = session
        .get_json_query::<NetworkService, _, SubnetsRoot>(
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
    session: &Session,
    query: &Q,
) -> Result<Vec<FloatingIp>> {
    trace!("Listing floating IPs with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, FloatingIpsRoot>(&["floatingips"], query, None)?
        .floatingips;
    trace!("Received floating IPs: {:?}", result);
    Ok(result)
}

/// List networks.
pub fn list_networks<Q: Serialize + Debug>(session: &Session, query: &Q) -> Result<Vec<Network>> {
    trace!("Listing networks with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, NetworksRoot>(&["networks"], query, None)?
        .networks;
    trace!("Received networks: {:?}", result);
    Ok(result)
}

/// List ports.
pub fn list_ports<Q: Serialize + Debug>(session: &Session, query: &Q) -> Result<Vec<Port>> {
    trace!("Listing ports with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, PortsRoot>(&["ports"], query, None)?
        .ports;
    trace!("Received ports: {:?}", result);
    Ok(result)
}

/// List subnets.
pub fn list_subnets<Q: Serialize + Debug>(session: &Session, query: &Q) -> Result<Vec<Subnet>> {
    trace!("Listing subnets with {:?}", query);
    let result = session
        .get_json_query::<NetworkService, _, SubnetsRoot>(&["subnets"], query, None)?
        .subnets;
    trace!("Received subnets: {:?}", result);
    Ok(result)
}

/// Update a floating IP.
pub fn update_floating_ip<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: FloatingIpUpdate,
) -> Result<FloatingIp> {
    debug!("Updating floatingIP {} with {:?}", id.as_ref(), update);
    let body = FloatingIpUpdateRoot { floatingip: update };
    let floating_ip = session
        .put_json::<NetworkService, _, FloatingIpRoot>(&["floatingips", id.as_ref()], body, None)?
        .floatingip;
    debug!("Updated floating IP {:?}", floating_ip);
    Ok(floating_ip)
}

/// Update a network.
pub fn update_network<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: NetworkUpdate,
) -> Result<Network> {
    debug!("Updating network {} with {:?}", id.as_ref(), update);
    let body = NetworkUpdateRoot { network: update };
    let network = session
        .put_json::<NetworkService, _, NetworkRoot>(&["networks", id.as_ref()], body, None)?
        .network;
    debug!("Updated network {:?}", network);
    Ok(network)
}

/// Update a port.
pub fn update_port<S: AsRef<str>>(session: &Session, id: S, update: PortUpdate) -> Result<Port> {
    debug!("Updating port {} with {:?}", id.as_ref(), update);
    let body = PortUpdateRoot { port: update };
    let port = session
        .put_json::<NetworkService, _, PortRoot>(&["ports", id.as_ref()], body, None)?
        .port;
    debug!("Updated port {:?}", port);
    Ok(port)
}

/// Update a subnet.
pub fn update_subnet<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: SubnetUpdate,
) -> Result<Subnet> {
    debug!("Updating subnet {} with {:?}", id.as_ref(), update);
    let body = SubnetUpdateRoot { subnet: update };
    let subnet = session
        .put_json::<NetworkService, _, SubnetRoot>(&["subnets", id.as_ref()], body, None)?
        .subnet;
    debug!("Updated subnet {:?}", subnet);
    Ok(subnet)
}
