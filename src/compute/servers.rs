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

//! Server management via Compute API.

use std::collections::HashMap;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;
use waiter::{Waiter, WaiterCurrentState};

use super::super::{Error, ErrorKind, Result, Sort};
use super::super::common::{self, DeletionWaiter, FlavorRef, ImageRef, KeyPairRef,
                           ListResources, NetworkRef, PortRef, ProjectRef,
                           Refresh, ResourceId, ResourceIterator, UserRef};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::protocol;


/// A query to server list.
#[derive(Clone, Debug)]
pub struct ServerQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a single server.
#[derive(Clone, Debug)]
pub struct Server {
    session: Rc<Session>,
    inner: protocol::Server
}

/// Structure representing a summary of a single server.
#[derive(Clone, Debug)]
pub struct ServerSummary {
    session: Rc<Session>,
    inner: common::protocol::IdAndName
}

/// Waiter for server status to change.
#[derive(Debug)]
pub struct ServerStatusWaiter<'server> {
    server: &'server mut Server,
    target: protocol::ServerStatus
}

/// A virtual NIC of a new server.
#[derive(Clone, Debug)]
pub enum ServerNIC {
    /// A NIC from the given network.
    FromNetwork(NetworkRef),
    /// A NIC with the given port.
    WithPort(PortRef),
    /// A NIC with the given fixed IP.
    WithFixedIp(Ipv4Addr)
}

/// A request to create a server.
#[derive(Debug)]
pub struct NewServer {
    session: Rc<Session>,
    flavor: FlavorRef,
    image: Option<ImageRef>,
    keypair: Option<KeyPairRef>,
    metadata: HashMap<String, String>,
    name: String,
    networks: Vec<ServerNIC>,
}

/// Waiter for server to be created.
#[derive(Debug)]
pub struct ServerCreationWaiter {
    server: Server
}


impl Refresh for Server {
    /// Refresh the server.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_server(&self.inner.id)?;
        Ok(())
    }
}

impl Server {
    /// Load a Server object.
    pub(crate) fn new<Id: AsRef<str>>(session: Rc<Session>, id: Id)
            -> Result<Server> {
        let inner = session.get_server(id)?;
        Ok(Server {
            session: session,
            inner: inner
        })
    }

    /// Get the IPv4 address.
    ///
    /// If not None, this address should be used to access the server instead
    /// of one from the `addresses` method.
    pub fn access_ipv4(&self) -> Option<Ipv4Addr> {
        self.inner.accessIPv4
    }

    /// Get the IPv6 address.
    ///
    /// If not None, this address should be used to access the server instead
    /// of one from the `addresses` method.
    pub fn access_ipv6(&self) -> Option<Ipv6Addr> {
        self.inner.accessIPv6
    }

    /// Get a reference to associated addresses.
    pub fn addresses(&self) -> &HashMap<String, Vec<protocol::ServerAddress>> {
        &self.inner.addresses
    }

    /// Get a reference to the availability zone.
    pub fn availability_zone(&self) -> &String {
        &self.inner.availability_zone
    }

    /// Get the creation date and time.
    pub fn created_at(&self) -> DateTime<FixedOffset> {
        self.inner.created
    }

    /// Find a floating IP, if it exists.
    ///
    /// If multiple floating IPs exist, the first is returned.
    pub fn floating_ip(&self) -> Option<IpAddr> {
        self.inner.addresses.values()
            .flat_map(|l| l.iter())
            .filter(|a| a.addr_type == Some(protocol::AddressType::Floating))
            .map(|a| a.addr).next()
    }

    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to the image.
    ///
    /// May be None if the server was created from a volume.
    pub fn image_id(&self) -> Option<&String> {
        match self.inner.image {
            Some(ref image) => Some(&image.id),
            None => None
        }
    }

    /// Whether the server has an image.
    ///
    /// May return `false` if the server was created from a volume.
    pub fn has_image(&self) -> bool {
        self.inner.image.is_some()
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get a reference to associated metadata.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.inner.metadata
    }

    /// Get server power state.
    pub fn power_state(&self) -> protocol::ServerPowerState {
        self.inner.power_state
    }

    /// Get server status.
    pub fn status(&self) -> protocol::ServerStatus {
        self.inner.status
    }

    /// Get the last update date and time.
    pub fn updated_at(&self) -> DateTime<FixedOffset> {
        self.inner.updated
    }

    /// Delete the server.
    pub fn delete(self) -> Result<DeletionWaiter<Server>> {
        self.session.delete_server(&self.inner.id)?;
        Ok(DeletionWaiter::new(self, Duration::new(120, 0), Duration::new(1, 0)))
    }

    /// Reboot the server.
    pub fn reboot<'server>(&'server mut self, reboot_type: protocol::RebootType)
            -> Result<ServerStatusWaiter<'server>> {
        let mut args = HashMap::new();
        let _ = args.insert("type", reboot_type);
        self.session.server_action_with_args(&self.inner.id, "reboot", args)?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::Active
        })
    }

    /// Start the server, optionally wait for it to be active.
    pub fn start<'server>(&'server mut self)
            -> Result<ServerStatusWaiter<'server>> {
        self.session.server_simple_action(&self.inner.id, "os-start")?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::Active
        })
    }

    /// Stop the server, optionally wait for it to be powered off.
    pub fn stop<'server>(&'server mut self)
            -> Result<ServerStatusWaiter<'server>> {
        self.session.server_simple_action(&self.inner.id, "os-stop")?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::ShutOff
        })
    }
}

impl<'server> Waiter<(), Error> for ServerStatusWaiter<'server> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        // TODO(dtantsur): vary depending on target?
        Some(Duration::new(600, 0))
    }

    fn default_delay(&self) -> Duration {
        Duration::new(1, 0)
    }

    fn timeout_error(&self) -> Error {
        Error::new(ErrorKind::OperationTimedOut,
                   format!("Timeout waiting for server {} to reach state {}",
                           self.server.id(), self.target))
    }

    fn poll(&mut self) -> Result<Option<()>> {
        self.server.refresh()?;
        if self.server.status() == self.target {
            debug!("Server {} reached state {}", self.server.id(), self.target);
            Ok(Some(()))
        } else if self.server.status() == protocol::ServerStatus::Error {
            debug!("Failed to move server {} to {} - status is ERROR",
                   self.server.id(), self.target);
            Err(Error::new(ErrorKind::OperationFailed,
                           format!("Server {} got into ERROR state",
                                   self.server.id())))
        } else {
            trace!("Still waiting for server {} to get to state {}, current is {}",
                   self.server.id(), self.target, self.server.status());
            Ok(None)
        }
    }
}

impl<'server> WaiterCurrentState<Server> for ServerStatusWaiter<'server> {
    fn waiter_current_state(&self) -> &Server {
        &self.server
    }
}

impl ServerSummary {
    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(&self) -> Result<Server> {
        Server::new(self.session.clone(), &self.inner.id)
    }

    /// Delete the server.
    pub fn delete(self) -> Result<()> {
        // TODO(dtantsur): implement wait
        self.session.delete_server(&self.inner.id)
    }
}

impl ServerQuery {
    pub(crate) fn new(session: Rc<Session>) -> ServerQuery {
        ServerQuery {
            session: session,
            query: Query::new(),
            can_paginate: true,
        }
    }

    /// Add marker to the request.
    ///
    /// Using this disables automatic pagination.
    pub fn with_marker<T: Into<String>>(mut self, marker: T) -> Self {
        self.can_paginate = false;
        self.query.push_str("marker", marker);
        self
    }

    /// Add limit to the request.
    ///
    /// Using this disables automatic pagination.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.can_paginate = false;
        self.query.push("limit", limit);
        self
    }

    /// Add sorting to the request.
    pub fn sort_by(mut self, sort: Sort<protocol::ServerSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.query.push_str("sort_key", field);
        self.query.push("sort_dir", direction);
        self
    }

    /// Filter by IPv4 address that should be used to access the server.
    pub fn with_access_ip_v4<T: Into<Ipv4Addr>>(mut self, value: T) -> Self {
        self.query.push("access_ip_v4", value.into());
        self
    }

    /// Filter by IPv6 address that should be used to access the server.
    pub fn with_access_ip_v6<T: Into<Ipv6Addr>>(mut self, value: T) -> Self {
        self.query.push("access_ipv6", value.into());
        self
    }

    /// Filter by availability zone.
    pub fn with_availability_zone<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("availability_zone", value);
        self
    }

    /// Filter by flavor.
    pub fn with_flavor<T: Into<FlavorRef>>(mut self, value: T) -> Self {
        self.query.push_str("flavor", value.into());
        self
    }

    /// Filter by host name.
    pub fn with_hostname<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("hostname", value);
        self
    }

    /// Filter by image ID.
    pub fn with_image<T: Into<ImageRef>>(mut self, value: T) -> Self {
        self.query.push_str("image", value.into());
        self
    }

    /// Filter by an IPv4 address.
    pub fn with_ip_v4<T: Into<Ipv4Addr>>(mut self, value: T) -> Self {
        self.query.push("ip", value.into());
        self
    }

    /// Filter by an IPv6 address.
    pub fn with_ip_v6<T: Into<Ipv6Addr>>(mut self, value: T) -> Self {
        self.query.push("ip6", value.into());
        self
    }

    /// Filter by server name (a database regular expression).
    pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("name", value);
        self
    }

    /// Filter by project ID (also commonly known as tenant ID).
    pub fn with_project<T: Into<ProjectRef>>(mut self, value: T) -> Self {
        self.query.push_str("project_id", value.into());
        self
    }

    /// Filter by server status.
    pub fn with_status(mut self, value: protocol::ServerStatus) -> Self {
        self.query.push_str("status", value.to_string());
        self
    }

    /// Filter by user ID.
    pub fn with_user<T: Into<UserRef>>(mut self, value: T) -> Self {
        self.query.push_str("user_id", value.into());
        self
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// This iterator yields only `ServerSummary` objects, containing
    /// IDs and names. Use `into_iter_detailed` for full `Server` objects.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<ServerSummary> {
        debug!("Fetching servers with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// This iterator yields full `Server` objects. If you only need IDs
    /// and/or names, use `into_iter` to save bandwidth.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter_detailed(self) -> ResourceIterator<Server> {
        debug!("Fetching server details with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<ServerSummary>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<ServerSummary> {
        debug!("Fetching one server with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

fn convert_networks(session: &Session, networks: Vec<ServerNIC>)
        -> Result<Vec<protocol::ServerNetwork>> {
    let mut result = Vec::with_capacity(networks.len());
    for item in networks {
        result.push(match item {
            ServerNIC::FromNetwork(n) => protocol::ServerNetwork::Network {
                uuid: n.into_verified(session)?
            },
            ServerNIC::WithPort(p) =>
                protocol::ServerNetwork::Port { port: p.into() },
            ServerNIC::WithFixedIp(ip) =>
                protocol::ServerNetwork::FixedIp{ fixed_ip: ip }
        });
    }
    Ok(result)
}

impl NewServer {
    /// Start creating a server.
    pub(crate) fn new(session: Rc<Session>, name: String, flavor: FlavorRef)
            -> NewServer {
        NewServer {
            session: session,
            flavor: flavor,
            image: None,
            keypair: None,
            metadata: HashMap::new(),
            name: name,
            networks: Vec::new(),
        }
    }

    /// Request creation of the server.
    pub fn create(self) -> Result<ServerCreationWaiter> {
        let request = protocol::ServerCreate {
            flavorRef: self.flavor.into_verified(&self.session)?,
            imageRef: match self.image {
                Some(img) => Some(img.into_verified(&self.session)?),
                None => None
            },
            key_name: match self.keypair {
                Some(item) => Some(item.into_verified(&self.session)?),
                None => None
            },
            metadata: self.metadata,
            name: self.name,
            networks: convert_networks(&self.session, self.networks)?
        };

        let server_ref = self.session.create_server(request)?;
        Ok(ServerCreationWaiter {
            server: Server::new(self.session, server_ref.id)?
        })
    }

    /// Add a virtual NIC with given fixed IP to the new server.
    ///
    /// A shorthand for `add_nic`.
    pub fn add_fixed_ip(&mut self, fixed_ip: Ipv4Addr) {
        self.add_nic(ServerNIC::WithFixedIp(fixed_ip));
    }

    /// Add a virtual NIC from this network to the new server.
    ///
    /// A shorthand for `add_nic`.
    pub fn add_network<N>(&mut self, network: N) where N: Into<NetworkRef> {
        self.add_nic(ServerNIC::FromNetwork(network.into()));
    }

    /// Add a virtual NIC to the new server.
    pub fn add_nic(&mut self, nic: ServerNIC) {
        self.networks.push(nic);
    }

    /// Add a virtual NIC with this port to the new server.
    ///
    /// A shorthand for `add_nic`.
    pub fn add_port<P>(&mut self, port: P) where P: Into<PortRef> {
        self.add_nic(ServerNIC::WithPort(port.into()));
    }

    /// Use this image as a source for the new server.
    pub fn set_image<I>(&mut self, image: I) where I: Into<ImageRef> {
        self.image = Some(image.into());
    }

    /// Use this key pair for the new server.
    pub fn set_keypair<K>(&mut self, keypair: K) where K: Into<KeyPairRef> {
        self.keypair = Some(keypair.into());
    }

    /// Add a virtual NIC with given fixed IP to the new server.
    pub fn with_fixed_ip(mut self, fixed_ip: Ipv4Addr) -> NewServer {
        self.add_fixed_ip(fixed_ip);
        self
    }

    /// Use this image as a source for the new server.
    pub fn with_image<I>(mut self, image: I) -> NewServer
            where I: Into<ImageRef> {
        self.set_image(image);
        self
    }

    /// Use this key pair for the new server.
    pub fn with_keypair<K>(mut self, keypair: K) -> NewServer
            where K: Into<KeyPairRef> {
        self.set_keypair(keypair);
        self
    }

    /// Add a virtual NIC from this network to the new server.
    pub fn with_network<N>(mut self, network: N) -> NewServer
            where N: Into<NetworkRef> {
        self.add_network(network);
        self
    }

    /// Add a virtual NIC with this port to the new server.
    pub fn with_port<P>(mut self, port: P) -> NewServer
            where P: Into<PortRef> {
        self.add_port(port);
        self
    }

    /// Add an arbitrary key/value metadata pair.
    pub fn with_metadata<S1, S2>(mut self, key: S1, value: S2) -> NewServer
            where S1: Into<String>,
                  S2: Into<String> {
        let _ = self.metadata.insert(key.into(), value.into());
        self
    }
}

impl Waiter<Server, Error> for ServerCreationWaiter {
    fn default_wait_timeout(&self) -> Option<Duration> {
        Some(Duration::new(1800, 0))
    }

    fn default_delay(&self) -> Duration {
        Duration::new(5, 0)
    }

    fn timeout_error(&self) -> Error {
        Error::new(ErrorKind::OperationTimedOut,
                   format!("Timeout waiting for server {} to become ACTIVE",
                           self.server.id()))
    }

    fn poll(&mut self) -> Result<Option<Server>> {
        self.server.refresh()?;
        if self.server.status() == protocol::ServerStatus::Active {
            debug!("Server {} successfully created", self.server.id());
            // TODO(dtantsur): get rid of clone?
            Ok(Some(self.server.clone()))
        } else if self.server.status() == protocol::ServerStatus::Error {
            debug!("Failed create server {} - status is ERROR",
                   self.server.id());
            Err(Error::new(ErrorKind::OperationFailed,
                           format!("Server {} got into ERROR state",
                                   self.server.id())))
        } else {
            trace!("Still waiting for server {} to become ACTIVE, current is {}",
                   self.server.id(), self.server.status());
            Ok(None)
        }
    }
}

impl WaiterCurrentState<Server> for ServerCreationWaiter {
    fn waiter_current_state(&self) -> &Server {
        &self.server
    }
}

impl ResourceId for ServerSummary {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for ServerSummary {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<ServerSummary>> {
        Ok(session.list_servers(&query)?.into_iter().map(|srv| ServerSummary {
            session: session.clone(),
            inner: srv
        }).collect())
    }
}

impl ResourceId for Server {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for Server {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<Server>> {
        Ok(session.list_servers_detail(&query)?.into_iter().map(|srv| Server {
            session: session.clone(),
            inner: srv
        }).collect())
    }
}

impl IntoFallibleIterator for ServerQuery {
    type Item = ServerSummary;

    type Error = Error;

    type IntoIter = ResourceIterator<ServerSummary>;

    fn into_fallible_iterator(self) -> ResourceIterator<ServerSummary> {
        self.into_iter()
    }
}
