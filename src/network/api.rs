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

use std::collections::HashMap;
use std::fmt::Debug;

use osauth::services::NETWORK;
use serde::Serialize;

use super::super::session::Session;
use super::super::utils::{self, ResultExt};
use super::super::{Error, ErrorKind, Result};
use super::protocol::*;

/// Add extra routes to a router.
pub fn add_extra_routes<S>(session: &Session, id: S, routes: Vec<HostRoute>) -> Result<()>
where
    S: AsRef<str>,
{
    trace!("Add extra routes {:?} to router {}", routes, id.as_ref());
    let mut body = HashMap::new();
    let _ = body.insert("router", Routes { routes });

    let _ = session.put(
        NETWORK,
        &["routers", id.as_ref(), "add_extraroutes"],
        body,
        None,
    )?;

    Ok(())
}

/// Remove extra routes from a router.
pub fn remove_extra_routes<S>(session: &Session, id: S, routes: Vec<HostRoute>) -> Result<()>
where
    S: AsRef<str>,
{
    trace!(
        "Remove extra routes {:?} from router {}",
        routes,
        id.as_ref()
    );
    let mut body = HashMap::new();
    let _ = body.insert("router", Routes { routes });

    let _ = session.put(
        NETWORK,
        &["routers", id.as_ref(), "remove_extraroutes"],
        body,
        None,
    )?;

    Ok(())
}

/// Add an interface to a router.
pub fn add_router_interface<S>(
    session: &Session,
    id: S,
    subnet_id: Option<S>,
    port_id: Option<S>,
) -> Result<()>
where
    S: AsRef<str>,
{
    let mut body = HashMap::new();
    match (&subnet_id, &port_id) {
        (Some(subnet_id), None) => {
            trace!(
                "Add subnet {} to router {}.",
                subnet_id.as_ref(),
                id.as_ref()
            );
            let _ = body.insert("subnet_id", subnet_id.as_ref());
        }
        (None, Some(port_id)) => {
            trace!("Add port {} to router {}.", port_id.as_ref(), id.as_ref());
            let _ = body.insert("port_id", port_id.as_ref());
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Requires either subnet_id or port_id but not both.",
            ));
        }
    }

    let _ = session.put(
        NETWORK,
        &["routers", id.as_ref(), "add_router_interface"],
        body,
        None,
    )?;

    debug!("Successfully added interface to router {}", id.as_ref());

    Ok(())
}

/// Create a floating IP.
pub fn create_floating_ip(session: &Session, request: FloatingIp) -> Result<FloatingIp> {
    debug!("Creating a new floating IP with {:?}", request);
    let body = FloatingIpRoot {
        floatingip: request,
    };
    let root: FloatingIpRoot = session.post_json(NETWORK, &["floatingips"], body, None)?;
    debug!("Created floating IP {:?}", root.floatingip);
    Ok(root.floatingip)
}

/// Create a network.
pub fn create_network(session: &Session, request: Network) -> Result<Network> {
    debug!("Creating a new network with {:?}", request);
    let body = NetworkRoot { network: request };
    let root: NetworkRoot = session.post_json(NETWORK, &["networks"], body, None)?;
    debug!("Created network {:?}", root.network);
    Ok(root.network)
}

/// Create a port.
pub fn create_port(session: &Session, request: Port) -> Result<Port> {
    debug!("Creating a new port with {:?}", request);
    let body = PortRoot { port: request };
    let root: PortRoot = session.post_json(NETWORK, &["ports"], body, None)?;
    debug!("Created port {:?}", root.port);
    Ok(root.port)
}

/// Create a router.
pub fn create_router(session: &Session, request: Router) -> Result<Router> {
    debug!("Creating a new router with {:?}", request);
    let body = RouterRoot { router: request };
    let root: RouterRoot = session.post_json(NETWORK, &["routers"], body, None)?;
    debug!("Created router {:?}", root.router);
    Ok(root.router)
}

/// Create a subnet.
pub fn create_subnet(session: &Session, request: Subnet) -> Result<Subnet> {
    debug!("Creating a new subnet with {:?}", request);
    let body = SubnetRoot { subnet: request };
    let root: SubnetRoot = session.post_json(NETWORK, &["subnets"], body, None)?;
    debug!("Created subnet {:?}", root.subnet);
    Ok(root.subnet)
}

/// Delete a floating IP.
pub fn delete_floating_ip<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting floating IP {}", id.as_ref());
    let _ = session.delete(NETWORK, &["floatingips", id.as_ref()], None)?;
    debug!("Floating IP {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a network.
pub fn delete_network<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting network {}", id.as_ref());
    let _ = session.delete(NETWORK, &["networks", id.as_ref()], None)?;
    debug!("Network {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a port.
pub fn delete_port<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting port {}", id.as_ref());
    let _ = session.delete(NETWORK, &["ports", id.as_ref()], None)?;
    debug!("Port {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a router.
pub fn delete_router<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting router {}", id.as_ref());
    let _ = session.delete(NETWORK, &["routers", id.as_ref()], None)?;
    debug!("Router {} was deleted", id.as_ref());
    Ok(())
}

/// Delete a subnet.
pub fn delete_subnet<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    debug!("Deleting subnet {}", id.as_ref());
    let _ = session.delete(NETWORK, &["subnets", id.as_ref()], None)?;
    debug!("Subnet {} was deleted", id.as_ref());
    Ok(())
}

/// Get a floating IP.
pub fn get_floating_ip<S: AsRef<str>>(session: &Session, id: S) -> Result<FloatingIp> {
    trace!("Get floating IP by ID {}", id.as_ref());
    let root: FloatingIpRoot = session.get_json(NETWORK, &["floatingips", id.as_ref()], None)?;
    trace!("Received {:?}", root.floatingip);
    Ok(root.floatingip)
}

/// Get a network.
pub fn get_network<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Network> {
    let s = id_or_name.as_ref();
    get_network_by_id(session, s).if_not_found_then(|| get_network_by_name(session, s))
}

/// Get a network by its ID.
pub fn get_network_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Network> {
    trace!("Get network by ID {}", id.as_ref());
    let root: NetworkRoot = session.get_json(NETWORK, &["networks", id.as_ref()], None)?;
    trace!("Received {:?}", root.network);
    Ok(root.network)
}

/// Get a network by its name.
pub fn get_network_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Network> {
    trace!("Get network by name {}", name.as_ref());
    let root: NetworksRoot =
        session.get_json_query(NETWORK, &["networks"], &[("name", name.as_ref())], None)?;
    let result = utils::one(
        root.networks,
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
    let root: PortRoot = session.get_json(NETWORK, &["ports", id.as_ref()], None)?;
    trace!("Received {:?}", root.port);
    Ok(root.port)
}

/// Get a port by its name.
pub fn get_port_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Port> {
    trace!("Get port by name {}", name.as_ref());
    let root: PortsRoot =
        session.get_json_query(NETWORK, &["ports"], &[("name", name.as_ref())], None)?;
    let result = utils::one(
        root.ports,
        "Port with given name or ID not found",
        "Too many ports found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Get a router.
pub fn get_router<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Router> {
    let s = id_or_name.as_ref();
    get_router_by_id(session, s).if_not_found_then(|| get_router_by_name(session, s))
}

/// Get a router by its ID.
pub fn get_router_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Router> {
    trace!("Get router by ID {}", id.as_ref());
    let root: RouterRoot = session.get_json(NETWORK, &["routers", id.as_ref()], None)?;
    trace!("Received {:?}", root.router);
    Ok(root.router)
}

/// Get a router by its name.
pub fn get_router_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Router> {
    trace!("Get router by name {}", name.as_ref());
    let root: RoutersRoot =
        session.get_json_query(NETWORK, &["routers"], &[("name", name.as_ref())], None)?;
    let result = utils::one(
        root.routers,
        "Router with given name or ID not found",
        "Too many routers found with given name",
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
    let root: SubnetRoot = session.get_json(NETWORK, &["subnets", id.as_ref()], None)?;
    trace!("Received {:?}", root.subnet);
    Ok(root.subnet)
}

/// Get a subnet by its name.
pub fn get_subnet_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Subnet> {
    trace!("Get subnet by name {}", name.as_ref());
    let root: SubnetsRoot =
        session.get_json_query(NETWORK, &["subnets"], &[("name", name.as_ref())], None)?;
    let result = utils::one(
        root.subnets,
        "Subnet with given name or ID not found",
        "Too many subnets found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// List floating IPs.
pub fn list_floating_ips<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<FloatingIp>> {
    trace!("Listing floating IPs with {:?}", query);
    let root: FloatingIpsRoot = session.get_json_query(NETWORK, &["floatingips"], query, None)?;
    trace!("Received floating IPs: {:?}", root.floatingips);
    Ok(root.floatingips)
}

/// List networks.
pub fn list_networks<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Network>> {
    trace!("Listing networks with {:?}", query);
    let root: NetworksRoot = session.get_json_query(NETWORK, &["networks"], query, None)?;
    trace!("Received networks: {:?}", root.networks);
    Ok(root.networks)
}

/// List ports.
pub fn list_ports<Q: Serialize + Sync + Debug>(session: &Session, query: &Q) -> Result<Vec<Port>> {
    trace!("Listing ports with {:?}", query);
    let root: PortsRoot = session.get_json_query(NETWORK, &["ports"], query, None)?;
    trace!("Received ports: {:?}", root.ports);
    Ok(root.ports)
}

/// List routers.
pub fn list_routers<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Router>> {
    trace!("Listing routers with {:?}", query);
    let root: RoutersRoot = session.get_json_query(NETWORK, &["routers"], query, None)?;
    trace!("Received routers: {:?}", root.routers);
    Ok(root.routers)
}

/// List subnets.
pub fn list_subnets<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Subnet>> {
    trace!("Listing subnets with {:?}", query);
    let root: SubnetsRoot = session.get_json_query(NETWORK, &["subnets"], query, None)?;
    trace!("Received subnets: {:?}", root.subnets);
    Ok(root.subnets)
}

/// Remove an interface from a router.
pub fn remove_router_interface<S>(
    session: &Session,
    id: S,
    subnet_id: Option<S>,
    port_id: Option<S>,
) -> Result<()>
where
    S: AsRef<str>,
{
    let mut body = HashMap::new();
    match (&subnet_id, &port_id) {
        (Some(subnet_id), None) => {
            trace!(
                "Remove subnet {} from router {}.",
                subnet_id.as_ref(),
                id.as_ref()
            );
            let _ = body.insert("subnet_id", subnet_id.as_ref());
        }
        (None, Some(port_id)) => {
            trace!(
                "Remove port {} from router {}.",
                port_id.as_ref(),
                id.as_ref()
            );
            let _ = body.insert("port_id", port_id.as_ref());
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Requires either subnet_id or port_id but not both.",
            ));
        }
    }

    let _ = session.put(
        NETWORK,
        &["routers", id.as_ref(), "remove_router_interface"],
        body,
        None,
    )?;

    debug!("Successfully removed interface to router {}", id.as_ref());

    Ok(())
}

/// Update a floating IP.
pub fn update_floating_ip<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: FloatingIpUpdate,
) -> Result<FloatingIp> {
    debug!("Updating floating IP {} with {:?}", id.as_ref(), update);
    let body = FloatingIpUpdateRoot { floatingip: update };
    let root: FloatingIpRoot =
        session.put_json(NETWORK, &["floatingips", id.as_ref()], body, None)?;
    debug!("Updated floating IP {:?}", root.floatingip);
    Ok(root.floatingip)
}

/// Update a network.
pub fn update_network<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: NetworkUpdate,
) -> Result<Network> {
    debug!("Updating network {} with {:?}", id.as_ref(), update);
    let body = NetworkUpdateRoot { network: update };
    let root: NetworkRoot = session.put_json(NETWORK, &["networks", id.as_ref()], body, None)?;
    debug!("Updated network {:?}", root.network);
    Ok(root.network)
}

/// Update a port.
pub fn update_port<S: AsRef<str>>(session: &Session, id: S, update: PortUpdate) -> Result<Port> {
    debug!("Updating port {} with {:?}", id.as_ref(), update);
    let body = PortUpdateRoot { port: update };
    let root: PortRoot = session.put_json(NETWORK, &["ports", id.as_ref()], body, None)?;
    debug!("Updated port {:?}", root.port);
    Ok(root.port)
}

/// Update a router.
pub fn update_router<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: RouterUpdate,
) -> Result<Router> {
    debug!("Updating router {} with {:?}", id.as_ref(), update);
    let body = RouterUpdateRoot { router: update };
    let root: RouterRoot = session.put_json(NETWORK, &["routers", id.as_ref()], body, None)?;
    debug!("Updated router {:?}", root.router);
    Ok(root.router)
}

/// Update a subnet.
pub fn update_subnet<S: AsRef<str>>(
    session: &Session,
    id: S,
    update: SubnetUpdate,
) -> Result<Subnet> {
    debug!("Updating subnet {} with {:?}", id.as_ref(), update);
    let body = SubnetUpdateRoot { subnet: update };
    let root: SubnetRoot = session.put_json(NETWORK, &["subnets", id.as_ref()], body, None)?;
    debug!("Updated subnet {:?}", root.subnet);
    Ok(root.subnet)
}
