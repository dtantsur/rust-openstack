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

use reqwest::{Method, Url};
use serde::Serialize;

use super::super::Result;
use super::super::auth::AuthMethod;
use super::super::common;
use super::super::session::{Session, ServiceInfo, ServiceType};
use super::super::utils::{self, ResultExt};
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Create a port.
    fn create_port(&self, request: protocol::Port) -> Result<protocol::Port>;

    /// Delete a port.
    fn delete_port<S: AsRef<str>>(&self, id_or_name: S) -> Result<()>;

    /// Delete a subnet.
    fn delete_subnet<S: AsRef<str>>(&self, id: S) -> Result<()>;

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

    /// List networks.
    fn list_networks<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Network>>;

    /// List ports.
    fn list_ports<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Port>>;

    /// List subnets.
    fn list_subnets<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Subnet>>;
}


/// Service type of Network API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "network";
const VERSION_ID: &'static str = "v2.0";


impl V2API for Session {
    fn create_port(&self, request: protocol::Port) -> Result<protocol::Port> {
        debug!("Creating a new port with {:?}", request);
        let body = protocol::PortRoot { port: request };
        let port = self.request::<V2>(Method::Post, &["ports"], None)?
            .json(&body).receive_json::<protocol::PortRoot>()?.port;
        debug!("Created port {:?}", port);
        Ok(port)
    }

    fn delete_port<S: AsRef<str>>(&self, id: S) -> Result<()> {
        debug!("Deleting port {}", id.as_ref());
        let _ = self.request::<V2>(Method::Delete,
                                   &["ports", id.as_ref()],
                                   None)?
            .send()?;
        debug!("Port {} was deleted", id.as_ref());
        Ok(())
    }

    fn delete_subnet<S: AsRef<str>>(&self, id: S) -> Result<()> {
        debug!("Deleting subnet {}", id.as_ref());
        let _ = self.request::<V2>(Method::Delete,
                                   &["subnets", id.as_ref()],
                                   None)?
            .send()?;
        debug!("Subnet {} was deleted", id.as_ref());
        Ok(())
    }

    fn get_network_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Network> {
        trace!("Get network by ID {}", id.as_ref());
        let network = self.request::<V2>(Method::Get,
                                         &["networks", id.as_ref()],
                                         None)?
           .receive_json::<protocol::NetworkRoot>()?.network;
        trace!("Received {:?}", network);
        Ok(network)
    }

    fn get_network_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Network> {
        trace!("Get network by name {}", name.as_ref());
        let items = self.request::<V2>(Method::Get, &["networks"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::NetworksRoot>()?.networks;
        let result = utils::one(items, "Network with given name or ID not found",
                                "Too many networks found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
    }

    fn get_port_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Port> {
        trace!("Get port by ID {}", id.as_ref());
        let port = self.request::<V2>(Method::Get,
                                         &["ports", id.as_ref()],
                                         None)?
           .receive_json::<protocol::PortRoot>()?.port;
        trace!("Received {:?}", port);
        Ok(port)
    }

    fn get_port_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Port> {
        trace!("Get port by name {}", name.as_ref());
        let items = self.request::<V2>(Method::Get, &["ports"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::PortsRoot>()?.ports;
        let result = utils::one(items, "Port with given name or ID not found",
                                "Too many ports found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
    }

    fn get_subnet_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Subnet> {
        trace!("Get subnet by ID {}", id.as_ref());
        let subnet = self.request::<V2>(Method::Get,
                                         &["subnets", id.as_ref()],
                                         None)?
           .receive_json::<protocol::SubnetRoot>()?.subnet;
        trace!("Received {:?}", subnet);
        Ok(subnet)
    }

    fn get_subnet_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Subnet> {
        trace!("Get subnet by name {}", name.as_ref());
        let items = self.request::<V2>(Method::Get, &["subnets"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::SubnetsRoot>()?.subnets;
        let result = utils::one(items, "Subnet with given name or ID not found",
                                "Too many subnets found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
    }

    fn list_networks<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Network>> {
        trace!("Listing networks with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["networks"], None)?
           .query(query).receive_json::<protocol::NetworksRoot>()?.networks;
        trace!("Received networks: {:?}", result);
        Ok(result)
    }

    fn list_ports<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Port>> {
        trace!("Listing ports with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["ports"], None)?
           .query(query).receive_json::<protocol::PortsRoot>()?.ports;
        trace!("Received ports: {:?}", result);
        Ok(result)
    }

    fn list_subnets<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Subnet>> {
        trace!("Listing subnets with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["subnets"], None)?
           .query(query).receive_json::<protocol::SubnetsRoot>()?.subnets;
        trace!("Received subnets: {:?}", result);
        Ok(result)
    }
}


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        common::protocol::fetch_service_info(endpoint, auth, SERVICE_TYPE, VERSION_ID)
    }
}
