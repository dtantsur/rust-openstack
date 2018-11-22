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

use super::super::Result;
use super::super::common::ApiVersion;
use super::super::session::{RequestBuilderExt, Session, ServiceType};
use super::super::utils::{self, ResultExt};
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Create a floating IP.
    fn create_floating_ip(&self, request: protocol::FloatingIp) -> Result<protocol::FloatingIp>;

    /// Create a network.
    fn create_network(&self, request: protocol::Network) -> Result<protocol::Network>;

    /// Create a port.
    fn create_port(&self, request: protocol::Port) -> Result<protocol::Port>;

    /// Create a subnet.
    fn create_subnet(&self, request: protocol::Subnet) -> Result<protocol::Subnet>;

    /// Delete a floating IP.
    fn delete_floating_ip<S: AsRef<str>>(&self, id: S) -> Result<()>;

    /// Delete a port.
    fn delete_port<S: AsRef<str>>(&self, id_or_name: S) -> Result<()>;

    /// Delete a network.
    fn delete_network<S: AsRef<str>>(&self, id: S) -> Result<()>;

    /// Delete a subnet.
    fn delete_subnet<S: AsRef<str>>(&self, id: S) -> Result<()>;

    /// Get a floating IP.
    fn get_floating_ip<S: AsRef<str>>(&self, id: S) -> Result<protocol::FloatingIp>;

    /// Get a network.
    fn get_network<S: AsRef<str>>(&self, id_or_name: S) -> Result<protocol::Network> {
        let s = id_or_name.as_ref();
        self.get_network_by_id(s).if_not_found_then(|| self.get_network_by_name(s))
    }

    /// Get a network by its ID.
    fn get_network_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Network>;

    /// Get a network by its name.
    fn get_network_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Network>;

    /// Get a port.
    fn get_port<S: AsRef<str>>(&self, id_or_name: S) -> Result<protocol::Port> {
        let s = id_or_name.as_ref();
        self.get_port_by_id(s).if_not_found_then(|| self.get_port_by_name(s))
    }

    /// Get a port by its ID.
    fn get_port_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Port>;

    /// Get a port by its name.
    fn get_port_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Port>;

    /// Get a subnet.
    fn get_subnet<S: AsRef<str>>(&self, id_or_name: S) -> Result<protocol::Subnet> {
        let s = id_or_name.as_ref();
        self.get_subnet_by_id(s).if_not_found_then(|| self.get_subnet_by_name(s))
    }

    /// Get a subnet by its ID.
    fn get_subnet_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Subnet>;

    /// Get a subnet by its name.
    fn get_subnet_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Subnet>;

    /// List floating IPs.
    fn list_floating_ips<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::FloatingIp>>;

    /// List networks.
    fn list_networks<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Network>>;

    /// List ports.
    fn list_ports<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Port>>;

    /// List subnets.
    fn list_subnets<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Subnet>>;

    /// Update a floating IP.
    fn update_floating_ip<S: AsRef<str>>(&self, id: S, update: protocol::FloatingIpUpdate)
        -> Result<protocol::FloatingIp>;

    /// Update a port.
    fn update_port<S: AsRef<str>>(&self, id: S, update: protocol::PortUpdate)
        -> Result<protocol::Port>;

    /// Update a subnet.
    fn update_subnet<S: AsRef<str>>(&self, id: S, update: protocol::SubnetUpdate)
        -> Result<protocol::Subnet>;
}


/// Service type of Network API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &str = "network";
const MAJOR_VERSION: ApiVersion = ApiVersion(2, 0);


impl V2API for Session {
    fn create_floating_ip(&self, request: protocol::FloatingIp) -> Result<protocol::FloatingIp> {
        debug!("Creating a new floating IP with {:?}", request);
        let body = protocol::FloatingIpRoot { floatingip: request };
        let floating_ip = self.post::<V2>(&["floatingips"], None)?
            .json(&body).receive_json::<protocol::FloatingIpRoot>()?.floatingip;
        debug!("Created floating IP {:?}", floating_ip);
        Ok(floating_ip)
    }

    fn create_network(&self, request: protocol::Network) -> Result<protocol::Network> {
        debug!("Creating a new network with {:?}", request);
        let body = protocol::NetworkRoot { network: request };
        let network = self.post::<V2>(&["networks"], None)?
            .json(&body).receive_json::<protocol::NetworkRoot>()?.network;
        debug!("Created network {:?}", network);
        Ok(network)
    }

    fn create_port(&self, request: protocol::Port) -> Result<protocol::Port> {
        debug!("Creating a new port with {:?}", request);
        let body = protocol::PortRoot { port: request };
        let port = self.post::<V2>(&["ports"], None)?
            .json(&body).receive_json::<protocol::PortRoot>()?.port;
        debug!("Created port {:?}", port);
        Ok(port)
    }

    fn create_subnet(&self, request: protocol::Subnet) -> Result<protocol::Subnet> {
        debug!("Creating a new subnet with {:?}", request);
        let body = protocol::SubnetRoot { subnet: request };
        let subnet = self.post::<V2>(&["subnets"], None)?
            .json(&body).receive_json::<protocol::SubnetRoot>()?.subnet;
        debug!("Created subnet {:?}", subnet);
        Ok(subnet)
    }

    fn delete_floating_ip<S: AsRef<str>>(&self, id: S) -> Result<()> {
        debug!("Deleting floating IP {}", id.as_ref());
        self.delete::<V2>(&["floatingips", id.as_ref()], None)?.commit()?;
        debug!("Floating IP {} was deleted", id.as_ref());
        Ok(())
    }

    fn delete_network<S: AsRef<str>>(&self, id: S) -> Result<()> {
        debug!("Deleting network {}", id.as_ref());
        self.delete::<V2>(&["networks", id.as_ref()], None)?.commit()?;
        debug!("Network {} was deleted", id.as_ref());
        Ok(())
    }

    fn delete_port<S: AsRef<str>>(&self, id: S) -> Result<()> {
        debug!("Deleting port {}", id.as_ref());
        self.delete::<V2>(&["ports", id.as_ref()], None)?.commit()?;
        debug!("Port {} was deleted", id.as_ref());
        Ok(())
    }

    fn delete_subnet<S: AsRef<str>>(&self, id: S) -> Result<()> {
        debug!("Deleting subnet {}", id.as_ref());
        self.delete::<V2>(&["subnets", id.as_ref()], None)?.commit()?;
        debug!("Subnet {} was deleted", id.as_ref());
        Ok(())
    }

    fn get_floating_ip<S: AsRef<str>>(&self, id: S) -> Result<protocol::FloatingIp> {
        trace!("Get floating IP by ID {}", id.as_ref());
        let floatingip = self.get::<V2>(&["floatingips", id.as_ref()], None)?
           .receive_json::<protocol::FloatingIpRoot>()?.floatingip;
        trace!("Received {:?}", floatingip);
        Ok(floatingip)
    }

    fn get_network_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Network> {
        trace!("Get network by ID {}", id.as_ref());
        let network = self.get::<V2>(&["networks", id.as_ref()], None)?
           .receive_json::<protocol::NetworkRoot>()?.network;
        trace!("Received {:?}", network);
        Ok(network)
    }

    fn get_network_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Network> {
        trace!("Get network by name {}", name.as_ref());
        let items = self.get::<V2>(&["networks"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::NetworksRoot>()?.networks;
        let result = utils::one(items, "Network with given name or ID not found",
                                "Too many networks found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
    }

    fn get_port_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Port> {
        trace!("Get port by ID {}", id.as_ref());
        let port = self.get::<V2>(&["ports", id.as_ref()], None)?
           .receive_json::<protocol::PortRoot>()?.port;
        trace!("Received {:?}", port);
        Ok(port)
    }

    fn get_port_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Port> {
        trace!("Get port by name {}", name.as_ref());
        let items = self.get::<V2>(&["ports"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::PortsRoot>()?.ports;
        let result = utils::one(items, "Port with given name or ID not found",
                                "Too many ports found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
    }

    fn get_subnet_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Subnet> {
        trace!("Get subnet by ID {}", id.as_ref());
        let subnet = self.get::<V2>(&["subnets", id.as_ref()], None)?
           .receive_json::<protocol::SubnetRoot>()?.subnet;
        trace!("Received {:?}", subnet);
        Ok(subnet)
    }

    fn get_subnet_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Subnet> {
        trace!("Get subnet by name {}", name.as_ref());
        let items = self.get::<V2>(&["subnets"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::SubnetsRoot>()?.subnets;
        let result = utils::one(items, "Subnet with given name or ID not found",
                                "Too many subnets found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
    }

    fn list_floating_ips<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::FloatingIp>> {
        trace!("Listing floating IPs with {:?}", query);
        let result = self.get::<V2>(&["floatingips"], None)?
           .query(query).receive_json::<protocol::FloatingIpsRoot>()?.floatingips;
        trace!("Received floating IPs: {:?}", result);
        Ok(result)
    }

    fn list_networks<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Network>> {
        trace!("Listing networks with {:?}", query);
        let result = self.get::<V2>(&["networks"], None)?
           .query(query).receive_json::<protocol::NetworksRoot>()?.networks;
        trace!("Received networks: {:?}", result);
        Ok(result)
    }

    fn list_ports<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Port>> {
        trace!("Listing ports with {:?}", query);
        let result = self.get::<V2>(&["ports"], None)?
           .query(query).receive_json::<protocol::PortsRoot>()?.ports;
        trace!("Received ports: {:?}", result);
        Ok(result)
    }

    fn list_subnets<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Subnet>> {
        trace!("Listing subnets with {:?}", query);
        let result = self.get::<V2>(&["subnets"], None)?
           .query(query).receive_json::<protocol::SubnetsRoot>()?.subnets;
        trace!("Received subnets: {:?}", result);
        Ok(result)
    }

    fn update_floating_ip<S: AsRef<str>>(&self, id: S, update: protocol::FloatingIpUpdate)
            -> Result<protocol::FloatingIp> {
        debug!("Updating floatingIP {} with {:?}", id.as_ref(), update);
        let body = protocol::FloatingIpUpdateRoot { floatingip: update };
        let floating_ip = self.put::<V2>(&["floatingips", id.as_ref()], None)?
            .json(&body).receive_json::<protocol::FloatingIpRoot>()?.floatingip;
        debug!("Updated floating IP {:?}", floating_ip);
        Ok(floating_ip)
    }

    fn update_port<S: AsRef<str>>(&self, id: S, update: protocol::PortUpdate)
            -> Result<protocol::Port> {
        debug!("Updating port {} with {:?}", id.as_ref(), update);
        let body = protocol::PortUpdateRoot { port: update };
        let port = self.put::<V2>(&["ports", id.as_ref()], None)?
            .json(&body).receive_json::<protocol::PortRoot>()?.port;
        debug!("Updated port {:?}", port);
        Ok(port)
    }

    fn update_subnet<S: AsRef<str>>(&self, id: S, update: protocol::SubnetUpdate)
            -> Result<protocol::Subnet> {
        debug!("Updating subnet {} with {:?}", id.as_ref(), update);
        let body = protocol::SubnetUpdateRoot { subnet: update };
        let subnet = self.put::<V2>(&["subnets", id.as_ref()], None)?
            .json(&body).receive_json::<protocol::SubnetRoot>()?.subnet;
        debug!("Updated subnet {:?}", subnet);
        Ok(subnet)
    }
}


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn major_version_supported(version: ApiVersion) -> bool {
        version == MAJOR_VERSION
    }
}
