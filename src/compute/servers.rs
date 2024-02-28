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
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use futures::stream::{Stream, TryStreamExt};
use osauth::common::IdAndName;
use serde::Serialize;

use super::super::common::{
    FlavorRef, ImageRef, KeyPairRef, NetworkRef, PortRef, ProjectRef, Refresh, ResourceIterator,
    ResourceQuery, UserRef, VolumeRef,
};
#[cfg(feature = "image")]
use super::super::image::Image;
use super::super::session::Session;
use super::super::utils::{unit_to_null, Query};
use super::super::waiter::{DeletionWaiter, Waiter};
use super::super::{Error, ErrorKind, Result, Sort};
use super::{api, protocol, BlockDevice, KeyPair};

/// A query to server list.
#[derive(Clone, Debug)]
pub struct ServerQuery {
    session: Session,
    query: Query,
    can_paginate: bool,
}

/// A detailed query to server list.
///
/// Is constructed from a `ServerQuery`.
#[derive(Clone, Debug)]
pub struct DetailedServerQuery {
    inner: ServerQuery,
}

/// Structure representing a single server.
#[derive(Clone, Debug)]
pub struct Server {
    session: Session,
    inner: protocol::Server,
}

/// Structure representing a summary of a single server.
#[derive(Clone, Debug)]
pub struct ServerSummary {
    session: Session,
    inner: IdAndName,
}

/// Waiter for server status to change.
#[derive(Debug)]
pub struct ServerStatusWaiter<'server> {
    server: &'server mut Server,
    target: protocol::ServerStatus,
}

/// A virtual NIC of a new server.
#[derive(Clone, Debug)]
pub enum ServerNIC {
    /// A NIC from the given network.
    FromNetwork(NetworkRef),
    /// A NIC with the given port.
    WithPort(PortRef),
    /// A NIC with the given fixed IP.
    WithFixedIp(Ipv4Addr),
}

/// A request to create a server.
#[derive(Debug)]
pub struct NewServer {
    session: Session,
    flavor: FlavorRef,
    image: Option<ImageRef>,
    keypair: Option<KeyPairRef>,
    metadata: HashMap<String, String>,
    name: String,
    nics: Vec<ServerNIC>,
    block_devices: Vec<BlockDevice>,
    user_data: Option<String>,
    config_drive: Option<bool>,
    availability_zone: Option<String>,
}

/// Waiter for server to be created.
#[derive(Debug)]
pub struct ServerCreationWaiter {
    server: Server,
}

#[async_trait]
impl Refresh for Server {
    /// Refresh the server.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_server_by_id(&self.session, &self.inner.id).await?;
        Ok(())
    }
}

impl Server {
    /// Create a new Server object.
    pub(crate) fn new(session: Session, inner: protocol::Server) -> Result<Server> {
        Ok(Server { session, inner })
    }

    /// Load a Server object.
    pub(crate) async fn load<Id: AsRef<str>>(session: Session, id: Id) -> Result<Server> {
        let inner = api::get_server(&session, id).await?;
        Server::new(session, inner)
    }

    transparent_property! {
        #[doc = "IPv4 address to access the server (if provided)."]
        access_ipv4: Option<Ipv4Addr>
    }

    transparent_property! {
        #[doc = "IPv6 address to access the server (if provided)."]
        access_ipv6: Option<Ipv6Addr>
    }

    transparent_property! {
        #[doc = "Addresses (floating and fixed) associated with the server."]
        addresses: ref HashMap<String, Vec<protocol::ServerAddress>>
    }

    transparent_property! {
        #[doc = "Availability zone."]
        availability_zone: ref String
    }

    transparent_property! {
        #[doc = "Creation date and time."]
        created_at: DateTime<FixedOffset>
    }

    transparent_property! {
        #[doc = "Server description."]
        description: ref Option<String>
    }

    /// Identifier of the flavor used to create this server.
    ///
    /// This is only known in old API versions, and the flavor is not guaranteed to exist any more.
    #[inline]
    pub fn flavor_id(&self) -> Option<&String> {
        match self.inner.flavor {
            protocol::AnyFlavor::Old(ref flavor) => Some(&flavor.id),
            _ => None,
        }
    }

    /// Flavor used to create this server.
    ///
    /// It may not possible to reconstruct a real Flavor object out of a Server, so this call
    /// returns the corresponding information instead.
    #[inline]
    pub async fn flavor(&self) -> Result<protocol::ServerFlavor> {
        match self.inner.flavor {
            protocol::AnyFlavor::Old(ref flavor) => {
                let flavor = api::get_flavor(&self.session, &flavor.id).await?;
                Ok(protocol::ServerFlavor {
                    ephemeral_size: flavor.ephemeral,
                    extra_specs: flavor.extra_specs,
                    original_name: flavor.name,
                    ram_size: flavor.ram,
                    root_size: flavor.disk,
                    swap_size: flavor.swap,
                    vcpu_count: flavor.vcpus,
                })
            }
            protocol::AnyFlavor::New(ref flavor) => Ok(flavor.clone()),
        }
    }

    /// Find a floating IP, if it exists.
    ///
    /// If multiple floating IPs exist, the first is returned.
    pub fn floating_ip(&self) -> Option<IpAddr> {
        self.inner
            .addresses
            .values()
            .flat_map(|l| l.iter())
            .filter(|a| a.addr_type == Some(protocol::AddressType::Floating))
            .map(|a| a.addr)
            .next()
    }

    transparent_property! {
        #[doc = "Whether the server was created with a config drive."]
        has_config_drive: bool
    }

    /// Whether the server has an image.
    ///
    /// May return `false` if the server was created from a volume.
    #[inline]
    pub fn has_image(&self) -> bool {
        self.inner.image.is_some()
    }

    transparent_property! {
        #[doc = "Server unique ID."]
        id: ref String
    }

    /// Fetch the associated image.
    ///
    /// Fails with `ResourceNotFound` if the server does not have an image.
    #[cfg(feature = "image")]
    pub async fn image(&self) -> Result<Image> {
        match self.inner.image {
            Some(ref image) => Image::new(self.session.clone(), &image.id).await,
            None => Err(Error::new(
                ErrorKind::ResourceNotFound,
                "No image associated with server",
            )),
        }
    }

    /// Get a reference to the image.
    ///
    /// May be None if the server was created from a volume.
    pub fn image_id(&self) -> Option<&String> {
        match self.inner.image {
            Some(ref image) => Some(&image.id),
            None => None,
        }
    }

    transparent_property! {
        #[doc = "Instance name."]
        instance_name: ref Option<String>
    }

    /// Fetch the key pair used for the server.
    pub async fn key_pair(&self) -> Result<KeyPair> {
        match self.inner.key_pair_name {
            Some(ref key_pair) => KeyPair::new(self.session.clone(), key_pair).await,
            None => Err(Error::new(
                ErrorKind::ResourceNotFound,
                "No key pair associated with server",
            )),
        }
    }

    transparent_property! {
        #[doc = "Name of a key pair used with this server (if any)."]
        key_pair_name: ref Option<String>
    }

    transparent_property! {
        #[doc = "Server name."]
        name: ref String
    }

    transparent_property! {
        #[doc = "Metadata associated with the server."]
        metadata: ref HashMap<String, String>
    }

    transparent_property! {
        #[doc = "Server power state."]
        power_state: protocol::ServerPowerState
    }

    transparent_property! {
        #[doc = "Server status."]
        status: protocol::ServerStatus
    }

    transparent_property! {
        #[doc = "Last update date and time."]
        updated_at: DateTime<FixedOffset>
    }

    /// Delete the server.
    pub async fn delete(self) -> Result<DeletionWaiter<Server>> {
        api::delete_server(&self.session, &self.inner.id).await?;
        Ok(DeletionWaiter::new(
            self,
            Duration::new(120, 0),
            Duration::new(1, 0),
        ))
    }

    /// Reboot the server.
    pub async fn reboot(
        &mut self,
        reboot_type: protocol::RebootType,
    ) -> Result<ServerStatusWaiter<'_>> {
        self.action(ServerAction::Reboot { reboot_type }).await?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::Active,
        })
    }

    /// Start the server, optionally wait for it to be active.
    pub async fn start(&mut self) -> Result<ServerStatusWaiter<'_>> {
        self.action(ServerAction::Start).await?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::Active,
        })
    }

    /// Stop the server, optionally wait for it to be powered off.
    pub async fn stop(&mut self) -> Result<ServerStatusWaiter<'_>> {
        self.action(ServerAction::Stop).await?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::ShutOff,
        })
    }

    /// Run an action on the server.
    pub async fn action(&mut self, action: ServerAction) -> Result<()> {
        api::server_action_with_args(&self.session, &self.inner.id, action).await
    }
}

/// An action to perform on a server.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
#[allow(missing_copy_implementations)]
pub enum ServerAction {
    /// Reboots a server.
    #[serde(rename = "reboot")]
    Reboot {
        /// The type of the reboot action.
        #[serde(rename = "type")]
        reboot_type: protocol::RebootType,
    },
    /// Starts a stopped server.
    #[serde(rename = "os-start", serialize_with = "unit_to_null")]
    Start,
    /// Stops a running server.
    #[serde(rename = "os-stop", serialize_with = "unit_to_null")]
    Stop,
}

#[async_trait]
impl<'server> Waiter<(), Error> for ServerStatusWaiter<'server> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        // TODO(dtantsur): vary depending on target?
        Some(Duration::new(600, 0))
    }

    fn default_delay(&self) -> Duration {
        Duration::new(1, 0)
    }

    fn timeout_error(&self) -> Error {
        Error::new(
            ErrorKind::OperationTimedOut,
            format!(
                "Timeout waiting for server {} to reach state {}",
                self.server.id(),
                self.target
            ),
        )
    }

    async fn poll(&mut self) -> Result<Option<()>> {
        self.server.refresh().await?;
        if self.server.status() == self.target {
            debug!("Server {} reached state {}", self.server.id(), self.target);
            Ok(Some(()))
        } else if self.server.status() == protocol::ServerStatus::Error {
            debug!(
                "Failed to move server {} to {} - status is ERROR",
                self.server.id(),
                self.target
            );
            Err(Error::new(
                ErrorKind::OperationFailed,
                format!("Server {} got into ERROR state", self.server.id()),
            ))
        } else {
            trace!(
                "Still waiting for server {} to get to state {}, current is {}",
                self.server.id(),
                self.target,
                self.server.status()
            );
            Ok(None)
        }
    }
}

impl<'server> ServerStatusWaiter<'server> {
    /// Current state of the server.
    pub fn current_state(&self) -> &Server {
        self.server
    }
}

impl ServerSummary {
    transparent_property! {
        #[doc = "Server unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Server name."]
        name: ref String
    }

    /// Get details.
    pub async fn details(&self) -> Result<Server> {
        Server::load(self.session.clone(), &self.inner.id).await
    }

    /// Delete the server.
    pub async fn delete(self) -> Result<()> {
        // TODO(dtantsur): implement wait
        api::delete_server(&self.session, &self.inner.id).await
    }
}

impl ServerQuery {
    pub(crate) fn new(session: Session) -> ServerQuery {
        ServerQuery {
            session,
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

    /// Add all tenants to the request.
    pub fn all_tenants(mut self) -> Self {
        self.query.push("all_tenants", true);
        self
    }

    query_filter! {
        #[doc = "Filter by IPv4 address that should be used to access the server."]
        set_access_ip_v4, with_access_ip_v4 -> access_ip_v4: Ipv4Addr
    }

    query_filter! {
        #[doc = "Filter by IPv6 address that should be used to access the server."]
        set_access_ip_v6, with_access_ip_v6 -> access_ip_v6: Ipv6Addr
    }

    query_filter! {
        #[doc = "Filter by availability zone."]
        set_availability_zone, with_availability_zone -> availability_zone: String
    }

    query_filter! {
        #[doc = "Filter by flavor."]
        set_flavor, with_flavor -> flavor: FlavorRef
    }

    query_filter! {
        #[doc = "Filter by host name."]
        set_hostname, with_hostname -> hostname: String
    }

    query_filter! {
        #[doc = "Filter by image used to build the server."]
        set_image, with_image -> image: ImageRef
    }

    query_filter! {
        #[doc = "Filter by an IPv4 address."]
        set_ip_v4, with_ip_v4 -> ip: Ipv4Addr
    }

    query_filter! {
        #[doc = "Filter by an IPv6 address."]
        set_ip_v6, with_ip_v6 -> ip6: Ipv6Addr
    }

    query_filter! {
        #[doc = "Filter by name."]
        set_name, with_name -> name: String
    }

    query_filter! {
        #[doc = "Filter by project (also commonly known as tenant)."]
        set_project, with_project -> project_id: ProjectRef
    }

    query_filter! {
        #[doc = "Filter by server status."]
        set_status, with_status -> status: protocol::ServerStatus
    }

    query_filter! {
        #[doc = "Filter by user."]
        set_user, with_user -> user_id: UserRef
    }

    /// Convert this query into a detailed query.
    ///
    /// Detailed queries return full `Server` objects instead of just `ServerSummary`.
    #[inline]
    pub fn detailed(self) -> DetailedServerQuery {
        DetailedServerQuery { inner: self }
    }

    /// Convert this query into a stream executing the request.
    ///
    /// This stream yields only `ServerSummary` objects, containing
    /// IDs and names. Use `detailed().into_stream()` for full `Server` objects.
    ///
    /// Returns a `TryStream`, which is a stream with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    #[inline]
    pub fn into_stream(self) -> impl Stream<Item = Result<ServerSummary>> {
        debug!("Fetching servers with {:?}", self.query);
        ResourceIterator::new(self).into_stream()
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_stream().try_collect().await`.
    #[inline]
    pub async fn all(self) -> Result<Vec<ServerSummary>> {
        self.into_stream().try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<ServerSummary> {
        debug!("Fetching one server with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        ResourceIterator::new(self).one().await
    }
}

#[async_trait]
impl ResourceQuery for ServerQuery {
    type Item = ServerSummary;

    const DEFAULT_LIMIT: usize = 100;

    async fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    async fn fetch_chunk(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_servers(&self.session, &query)
            .await?
            .into_iter()
            .map(|srv| ServerSummary {
                session: self.session.clone(),
                inner: srv,
            })
            .collect())
    }
}

impl DetailedServerQuery {
    /// Convert this query into a stream executing the request.
    ///
    /// This stream yields full `Server` objects.
    ///
    /// Returns a `TryStream`, which is a stream with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_stream(self) -> impl Stream<Item = Result<Server>> {
        debug!("Fetching server details with {:?}", self.inner.query);
        ResourceIterator::new(self).into_stream()
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_stream().try_collect().await`.
    #[inline]
    pub async fn all(self) -> Result<Vec<Server>> {
        self.into_stream().try_collect().await
    }
}

#[async_trait]
impl ResourceQuery for DetailedServerQuery {
    type Item = Server;

    const DEFAULT_LIMIT: usize = 50;

    async fn can_paginate(&self) -> Result<bool> {
        Ok(self.inner.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    async fn fetch_chunk(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Result<Vec<Self::Item>> {
        let query = self.inner.query.with_marker_and_limit(limit, marker);
        let servers = api::list_servers_detail(&self.inner.session, &query).await?;
        let mut result = Vec::with_capacity(servers.len());
        for srv in servers {
            result.push(Server::new(self.inner.session.clone(), srv)?);
        }
        Ok(result)
    }
}

impl From<DetailedServerQuery> for ServerQuery {
    fn from(value: DetailedServerQuery) -> ServerQuery {
        value.inner
    }
}

impl From<ServerQuery> for DetailedServerQuery {
    fn from(value: ServerQuery) -> DetailedServerQuery {
        value.detailed()
    }
}

async fn convert_networks(
    session: &Session,
    networks: Vec<ServerNIC>,
) -> Result<Vec<protocol::ServerNetwork>> {
    let mut result = Vec::with_capacity(networks.len());
    for item in networks {
        result.push(match item {
            ServerNIC::FromNetwork(n) => protocol::ServerNetwork::Network {
                uuid: n.into_verified(session).await?.into(),
            },
            ServerNIC::WithPort(p) => protocol::ServerNetwork::Port {
                port: p.into_verified(session).await?.into(),
            },
            ServerNIC::WithFixedIp(ip) => protocol::ServerNetwork::FixedIp { fixed_ip: ip },
        });
    }
    Ok(result)
}

impl NewServer {
    /// Start creating a server.
    pub(crate) fn new(session: Session, name: String, flavor: FlavorRef) -> NewServer {
        NewServer {
            session,
            flavor,
            image: None,
            keypair: None,
            metadata: HashMap::new(),
            name,
            nics: Vec::new(),
            block_devices: Vec::new(),
            user_data: None,
            config_drive: None,
            availability_zone: None,
        }
    }

    /// Request creation of the server.
    pub async fn create(self) -> Result<ServerCreationWaiter> {
        let mut block_devices = Vec::with_capacity(self.block_devices.len());
        for bd in self.block_devices {
            block_devices.push(bd.into_verified(&self.session).await?);
        }

        let request = protocol::ServerCreate {
            block_devices,
            flavorRef: self.flavor.into_verified(&self.session).await?.into(),
            imageRef: match self.image {
                Some(img) => Some(img.into_verified(&self.session).await?.into()),
                None => None,
            },
            key_name: match self.keypair {
                Some(item) => Some(item.into_verified(&self.session).await?.into()),
                None => None,
            },
            metadata: self.metadata,
            name: self.name,
            networks: convert_networks(&self.session, self.nics).await?,
            user_data: self.user_data,
            config_drive: self.config_drive,
            availability_zone: self.availability_zone,
        };

        let server_ref = api::create_server(&self.session, request).await?;
        Ok(ServerCreationWaiter {
            server: Server::load(self.session, server_ref.id).await?,
        })
    }

    /// Add a virtual NIC with given fixed IP to the new server.
    #[inline]
    pub fn add_fixed_ip(&mut self, fixed_ip: Ipv4Addr) {
        self.nics.push(ServerNIC::WithFixedIp(fixed_ip));
    }

    /// Add a virtual NIC from this network to the new server.
    #[inline]
    pub fn add_network<N>(&mut self, network: N)
    where
        N: Into<NetworkRef>,
    {
        self.nics.push(ServerNIC::FromNetwork(network.into()));
    }

    /// Add a virtual NIC with this port to the new server.
    #[inline]
    pub fn add_port<P>(&mut self, port: P)
    where
        P: Into<PortRef>,
    {
        self.nics.push(ServerNIC::WithPort(port.into()));
    }

    /// Metadata assigned to this server.
    #[inline]
    pub fn metadata(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }

    /// NICs to attach to this server.
    #[inline]
    pub fn nics(&mut self) -> &mut Vec<ServerNIC> {
        &mut self.nics
    }

    /// Block devices attached to the server.
    #[inline]
    pub fn block_devices(&mut self) -> &mut Vec<BlockDevice> {
        &mut self.block_devices
    }

    /// Use this image as a source for the new server.
    pub fn set_image<I>(&mut self, image: I)
    where
        I: Into<ImageRef>,
    {
        self.image = Some(image.into());
    }

    /// Use this key pair for the new server.
    pub fn set_keypair<K>(&mut self, keypair: K)
    where
        K: Into<KeyPairRef>,
    {
        self.keypair = Some(keypair.into());
    }

    /// Use this availability_zone for the new server.
    pub fn set_availability_zone<A>(&mut self, availability_zone: A)
    where
        A: Into<String>,
    {
        self.availability_zone = Some(availability_zone.into());
    }

    /// Add a block device to attach to the server.
    #[inline]
    pub fn with_block_device(mut self, block_device: BlockDevice) -> Self {
        self.block_devices.push(block_device);
        self
    }

    /// Add a volume to boot from.
    #[inline]
    pub fn with_boot_volume<V>(self, volume: V) -> Self
    where
        V: Into<VolumeRef>,
    {
        self.with_block_device(BlockDevice::from_volume(volume, true))
    }

    /// Add a virtual NIC with given fixed IP to the new server.
    #[inline]
    pub fn with_fixed_ip(mut self, fixed_ip: Ipv4Addr) -> NewServer {
        self.add_fixed_ip(fixed_ip);
        self
    }

    /// Use this image as a source for the new server.
    #[inline]
    pub fn with_image<I>(mut self, image: I) -> NewServer
    where
        I: Into<ImageRef>,
    {
        self.set_image(image);
        self
    }

    /// Use this key pair for the new server.
    #[inline]
    pub fn with_keypair<K>(mut self, keypair: K) -> NewServer
    where
        K: Into<KeyPairRef>,
    {
        self.set_keypair(keypair);
        self
    }

    /// Use this availability zone for the new server.
    #[inline]
    pub fn with_availability_zone<K>(mut self, availability_zone: K) -> NewServer
    where
        K: Into<String>,
    {
        self.set_availability_zone(availability_zone);
        self
    }

    /// Add an arbitrary key/value metadata pair.
    pub fn with_metadata<S1, S2>(mut self, key: S1, value: S2) -> NewServer
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        let _ = self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a virtual NIC from this network to the new server.
    #[inline]
    pub fn with_network<N>(mut self, network: N) -> NewServer
    where
        N: Into<NetworkRef>,
    {
        self.add_network(network);
        self
    }

    /// Create a volume to boot from from an image.
    #[inline]
    pub fn with_new_boot_volume<I>(self, image: I, size_gib: u32) -> Self
    where
        I: Into<ImageRef>,
    {
        self.with_block_device(BlockDevice::from_new_volume(image, size_gib, true))
    }

    /// Add a virtual NIC with this port to the new server.
    #[inline]
    pub fn with_port<P>(mut self, port: P) -> NewServer
    where
        P: Into<PortRef>,
    {
        self.add_port(port);
        self
    }

    creation_field! {
        #[doc = "Use this user-data for the new server."]
        set_user_data, with_user_data -> user_data: optional String
    }

    creation_field! {
        #[doc = "Enable/disable config-drive for the new server."]
        set_config_drive, with_config_drive -> config_drive: optional bool
    }
}

#[async_trait]
impl Waiter<Server, Error> for ServerCreationWaiter {
    fn default_wait_timeout(&self) -> Option<Duration> {
        Some(Duration::new(1800, 0))
    }

    fn default_delay(&self) -> Duration {
        Duration::new(5, 0)
    }

    fn timeout_error(&self) -> Error {
        Error::new(
            ErrorKind::OperationTimedOut,
            format!(
                "Timeout waiting for server {} to become ACTIVE",
                self.server.id()
            ),
        )
    }

    async fn poll(&mut self) -> Result<Option<Server>> {
        self.server.refresh().await?;
        if self.server.status() == protocol::ServerStatus::Active {
            debug!("Server {} successfully created", self.server.id());
            // TODO(dtantsur): get rid of clone?
            Ok(Some(self.server.clone()))
        } else if self.server.status() == protocol::ServerStatus::Error {
            debug!(
                "Failed create server {} - status is ERROR",
                self.server.id()
            );
            Err(Error::new(
                ErrorKind::OperationFailed,
                format!("Server {} got into ERROR state", self.server.id()),
            ))
        } else {
            trace!(
                "Still waiting for server {} to become ACTIVE, current is {}",
                self.server.id(),
                self.server.status()
            );
            Ok(None)
        }
    }
}

impl ServerCreationWaiter {
    /// Current state of the waiter.
    pub fn current_state(&self) -> &Server {
        &self.server
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_action_json() {
        assert_eq!(
            serde_json::to_string(&ServerAction::Start).unwrap(),
            "{\"os-start\":null}"
        );
        assert_eq!(
            serde_json::to_string(&ServerAction::Stop).unwrap(),
            "{\"os-stop\":null}"
        );
        assert_eq!(
            serde_json::to_string(&ServerAction::Reboot {
                reboot_type: protocol::RebootType::Hard
            })
            .unwrap(),
            "{\"reboot\":{\"type\":\"HARD\"}}"
        );
    }
}
