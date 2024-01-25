// Copyright 2018-2019 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Cloud API.

#[allow(unused_imports)]
use futures::io::AsyncRead;
#[allow(unused_imports)]
use std::io;

use super::auth::AuthType;
#[allow(unused_imports)]
use super::common::{ContainerRef, FlavorRef, NetworkRef};
#[cfg(feature = "block-storage")]
use super::block_storage::{Volume, VolumeQuery};
#[cfg(feature = "compute")]
use super::compute::{
    Flavor, FlavorQuery, FlavorSummary, KeyPair, KeyPairQuery, NewKeyPair, NewServer, Server,
    ServerQuery, ServerSummary,
};
#[cfg(feature = "image")]
use super::image::{Image, ImageQuery};
#[cfg(feature = "network")]
use super::network::{
    FloatingIp, FloatingIpQuery, Network, NetworkQuery, NewFloatingIp, NewNetwork, NewPort,
    NewRouter, NewSubnet, Port, PortQuery, Router, RouterQuery, Subnet, SubnetQuery,
};
#[cfg(feature = "object-storage")]
use super::object_storage::{Container, ContainerQuery, NewObject, Object, ObjectQuery};
use super::session::Session;
use super::{EndpointFilters, InterfaceType, Result};

/// OpenStack cloud API.
///
/// Provides high-level API for working with OpenStack clouds.
#[derive(Debug, Clone)]
pub struct Cloud {
    session: Session,
}

impl Cloud {
    /// Create a new cloud object with a given authentication plugin.
    ///
    /// See [`auth` module](auth/index.html) for details on how to authenticate
    /// against OpenStack clouds.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// async fn cloud() -> openstack::Result<openstack::Cloud> {
    ///     let scope = openstack::auth::Scope::Project {
    ///         project: openstack::IdOrName::from_name("project1"),
    ///         domain: Some(openstack::IdOrName::from_name("Default")),
    ///     };
    ///     let auth = openstack::auth::Password::new(
    ///             "https://cloud.example.com",
    ///             "user1", "pa$$word", "Default")
    ///         .expect("Invalid authentication URL")
    ///         .with_scope(scope);
    ///     openstack::Cloud::new(auth).await
    /// }
    /// ```
    ///
    /// # See Also
    ///
    /// * [from_config](#method.from_config) to create a Cloud from clouds.yaml
    /// * [from_env](#method.from_env) to create a Cloud from environment variables
    pub async fn new<Auth: AuthType + 'static>(auth_type: Auth) -> Result<Cloud> {
        Ok(Cloud {
            session: Session::new(auth_type).await?,
        })
    }

    /// Create a new cloud object from a configuration file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn cloud_from_config() -> openstack::Result<()> {
    /// let os = openstack::Cloud::from_config("cloud-1").await?;
    /// # Ok(()) }
    /// ```
    pub async fn from_config<S: AsRef<str>>(cloud_name: S) -> Result<Cloud> {
        Ok(Cloud {
            session: Session::from_config(cloud_name).await?,
        })
    }

    /// Create a new cloud object from environment variables.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn cloud_from_env() -> openstack::Result<()> {
    /// let os = openstack::Cloud::from_env().await?;
    /// # Ok(()) }
    /// ```
    pub async fn from_env() -> Result<Cloud> {
        Ok(Cloud {
            session: Session::from_env().await?,
        })
    }

    /// Endpoint filters for this cloud.
    #[inline]
    pub fn endpoint_filters(&self) -> &EndpointFilters {
        self.session.endpoint_filters()
    }

    /// Modify endpoint filters for this cloud.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// async fn cloud_from_env() -> openstack::Result<openstack::Cloud> {
    ///     let mut cloud = openstack::Cloud::from_env().await?;
    ///     {
    ///         let mut filters = cloud.endpoint_filters_mut();
    ///         filters.set_region("internal-1");
    ///         // Give priority to internal endpoints.
    ///         filters.set_interfaces(&[
    ///             openstack::InterfaceType::Internal,
    ///             openstack::InterfaceType::Public,
    ///         ][..])
    ///     }
    ///     Ok(cloud)
    /// }
    /// ```
    ///
    /// Removes cached endpoint information and detaches this object from a shared `Session`.
    pub fn endpoint_filters_mut(&mut self) -> &mut EndpointFilters {
        self.session.endpoint_filters_mut()
    }

    /// Convert this cloud into one using the given endpoint interface.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// async fn cloud_from_env() -> openstack::Result<openstack::Cloud> {
    ///     openstack::Cloud::from_env().await
    ///         .map(|os| os.with_endpoint_interface(openstack::InterfaceType::Internal))
    /// }
    /// ```
    ///
    /// Removes cached endpoint information and detaches this object from a shared `Session`.
    pub fn with_endpoint_interface(mut self, endpoint_interface: InterfaceType) -> Cloud {
        self.session.set_endpoint_interface(endpoint_interface);
        self
    }

    /// Convert this cloud into one using the given endpoint filters.
    ///
    /// Removes cached endpoint information and detaches this object from a shared `Session`.
    #[inline]
    pub fn with_endpoint_filters(mut self, endpoint_filters: EndpointFilters) -> Cloud {
        *self.endpoint_filters_mut() = endpoint_filters;
        self
    }

    /// Refresh this `Cloud` object (renew token, refetch service catalog, etc).
    pub async fn refresh(&mut self) -> Result<()> {
        self.session.refresh().await
    }

    /// Create a new container.
    ///
    /// If the container already exists, this call returns successfully.
    #[cfg(feature = "object-storage")]
    pub async fn create_container<Id: AsRef<str>>(&self, name: Id) -> Result<Container> {
        Container::create(self.session.clone(), name).await
    }

    /// Create a new object.
    #[cfg(feature = "object-storage")]
    pub async fn create_object<C, Id, R>(&self, container: C, name: Id, body: R) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
        R: AsyncRead + Send + Sync + 'static,
    {
        Object::create(self.session.clone(), container, name, body).await
    }

    /// Build a query against container list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "object-storage")]
    #[inline]
    pub fn find_containers(&self) -> ContainerQuery {
        ContainerQuery::new(self.session.clone())
    }

    /// Build a query against object list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "object-storage")]
    pub fn find_objects<C>(&self, container: C) -> ObjectQuery
    where
        C: Into<ContainerRef>,
    {
        ObjectQuery::new(self.session.clone(), container)
    }

    /// Build a query against flavor list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "compute")]
    pub fn find_flavors(&self) -> FlavorQuery {
        FlavorQuery::new(self.session.clone())
    }

    /// Build a query against floating IP list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "network")]
    pub fn find_floating_ips(&self) -> FloatingIpQuery {
        FloatingIpQuery::new(self.session.clone())
    }

    /// Build a query against image list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "image")]
    pub fn find_images(&self) -> ImageQuery {
        ImageQuery::new(self.session.clone())
    }

    /// Build a query against key pairs list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "compute")]
    pub fn find_keypairs(&self) -> KeyPairQuery {
        KeyPairQuery::new(self.session.clone())
    }

    /// Build a query against network list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "network")]
    pub fn find_networks(&self) -> NetworkQuery {
        NetworkQuery::new(self.session.clone())
    }

    /// Build a query against port list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "network")]
    pub fn find_ports(&self) -> PortQuery {
        PortQuery::new(self.session.clone())
    }

    /// Build a query against router list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "network")]
    pub fn find_routers(&self) -> RouterQuery {
        RouterQuery::new(self.session.clone())
    }

    /// Build a query against server list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    ///
    /// # Example
    ///
    /// Sorting servers by `access_ip_v4` and getting first 5 results:
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let sorting = openstack::compute::ServerSortKey::AccessIpv4;
    /// let server_list = os.find_servers()
    ///     .sort_by(openstack::Sort::Asc(sorting)).with_limit(5)
    ///     .all().await.expect("Unable to fetch servers");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub fn find_servers(&self) -> ServerQuery {
        ServerQuery::new(self.session.clone())
    }

    /// Build a query against subnet list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "network")]
    pub fn find_subnets(&self) -> SubnetQuery {
        SubnetQuery::new(self.session.clone())
    }

    /// Build a query against volume list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "block-storage")]
    pub fn find_volumes(&self) -> VolumeQuery {
        VolumeQuery::new(self.session.clone())
    }

    /// Get object container metadata by its name.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let ctr = os.get_container("www").await.expect("Unable to get a container");
    /// # }
    /// ```
    #[cfg(feature = "object-storage")]
    pub async fn get_container<Id: AsRef<str>>(&self, name: Id) -> Result<Container> {
        Container::load(self.session.clone(), name).await
    }

    /// Get object metadata by its name.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let obj = os.get_object("www", "/foo/bar").await.expect("Unable to get an object");
    /// # }
    /// ```
    #[cfg(feature = "object-storage")]
    pub async fn get_object<C, Id>(&self, container: C, name: Id) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
    {
        Object::load(self.session.clone(), container, name).await
    }

    /// Find a flavor by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_flavor("m1.medium").await.expect("Unable to get a flavor");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub async fn get_flavor<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Flavor> {
        Flavor::load(self.session.clone(), id_or_name).await
    }

    /// Find a floating IP by its ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_floating_ip("031e08c7-2ca7-4c0b-9923-030c8d946ba4")
    ///     .await
    ///     .expect("Unable to get a floating IP");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn get_floating_ip<Id: AsRef<str>>(&self, id: Id) -> Result<FloatingIp> {
        FloatingIp::load(self.session.clone(), id).await
    }

    /// Find an image by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_image("centos7").await.expect("Unable to get a image");
    /// # }
    /// ```
    #[cfg(feature = "image")]
    pub async fn get_image<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Image> {
        Image::new(self.session.clone(), id_or_name).await
    }

    /// Find a key pair by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_keypair("default").await.expect("Unable to get a key pair");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub async fn get_keypair<Id: AsRef<str>>(&self, name: Id) -> Result<KeyPair> {
        KeyPair::new(self.session.clone(), name).await
    }

    /// Find an network by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_network("centos7").await.expect("Unable to get a network");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn get_network<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Network> {
        Network::load(self.session.clone(), id_or_name).await
    }

    /// Find an port by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_port("4d9c1710-fa02-49f9-8218-291024ef4140")
    ///     .await
    ///     .expect("Unable to get a port");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn get_port<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Port> {
        Port::load(self.session.clone(), id_or_name).await
    }

    /// Find a router by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let router = os.get_router("router_name").await.expect("Unable to get a router");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn get_router<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Router> {
        Router::load(self.session.clone(), id_or_name).await
    }

    /// Find a server by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_server("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
    ///     .await
    ///     .expect("Unable to get a server");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub async fn get_server<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Server> {
        Server::load(self.session.clone(), id_or_name).await
    }

    /// Find an subnet by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server = os.get_subnet("private-subnet")
    ///     .await
    ///     .expect("Unable to get a subnet");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn get_subnet<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Subnet> {
        Subnet::load(self.session.clone(), id_or_name).await
    }

    /// List all containers.
    ///
    /// This call can yield a lot of results, use the
    /// [find_containers](#method.find_containers) call to limit the number of
    /// containers to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_containers().await.expect("Unable to fetch containers");
    /// # }
    /// ```
    #[cfg(feature = "object-storage")]
    pub async fn list_containers(&self) -> Result<Vec<Container>> {
        self.find_containers().all().await
    }

    /// List all objects.
    ///
    /// This call can yield a lot of results, use the
    /// [find_objects](#method.find_objects) call to limit the number of
    /// objects to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_objects("www").await.expect("Unable to fetch objects");
    /// # }
    /// ```
    #[cfg(feature = "object-storage")]
    pub async fn list_objects<C>(&self, container: C) -> Result<Vec<Object>>
    where
        C: Into<ContainerRef>,
    {
        self.find_objects(container).all().await
    }

    /// List all flavors.
    ///
    /// This call can yield a lot of results, use the
    /// [find_flavors](#method.find_flavors) call to limit the number of
    /// flavors to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_flavors().await.expect("Unable to fetch flavors");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub async fn list_flavors(&self) -> Result<Vec<FlavorSummary>> {
        self.find_flavors().all().await
    }

    /// List all floating IPs
    ///
    /// This call can yield a lot of results, use the
    /// [find_floating_ips](#method.find_floating_ips) call to limit the number of
    /// networks to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_floating_ips().await.expect("Unable to fetch floating IPs");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn list_floating_ips(&self) -> Result<Vec<FloatingIp>> {
        self.find_floating_ips().all().await
    }

    /// List all images.
    ///
    /// This call can yield a lot of results, use the
    /// [find_images](#method.find_images) call to limit the number of
    /// images to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_images().await.expect("Unable to fetch images");
    /// # }
    /// ```
    #[cfg(feature = "image")]
    pub async fn list_images(&self) -> Result<Vec<Image>> {
        self.find_images().all().await
    }

    /// List all key pairs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let result = os.list_keypairs().await.expect("Unable to fetch key pairs");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub async fn list_keypairs(&self) -> Result<Vec<KeyPair>> {
        self.find_keypairs().all().await
    }

    /// List all networks.
    ///
    /// This call can yield a lot of results, use the
    /// [find_networks](#method.find_networks) call to limit the number of
    /// networks to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_networks().await.expect("Unable to fetch networks");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn list_networks(&self) -> Result<Vec<Network>> {
        self.find_networks().all().await
    }

    /// List all ports.
    ///
    /// This call can yield a lot of results, use the
    /// [find_ports](#method.find_ports) call to limit the number of
    /// ports to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_ports().await.expect("Unable to fetch ports");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn list_ports(&self) -> Result<Vec<Port>> {
        self.find_ports().all().await
    }

    /// List all routers.
    ///
    /// This call can yield a lot of results, use the
    /// [find_routers](#method.find_routers) call to limit the number of
    /// routers to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let router_list = os.list_routers().await.expect("Unable to fetch routers");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn list_routers(&self) -> Result<Vec<Router>> {
        self.find_routers().all().await
    }

    /// List all servers.
    ///
    /// This call can yield a lot of results, use the
    /// [find_servers](#method.find_servers) call to limit the number of
    /// servers to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_servers().await.expect("Unable to fetch servers");
    /// # }
    /// ```
    #[cfg(feature = "compute")]
    pub async fn list_servers(&self) -> Result<Vec<ServerSummary>> {
        self.find_servers().all().await
    }

    /// List all subnets.
    ///
    /// This call can yield a lot of results, use the
    /// [find_subnets](#method.find_subnets) call to limit the number of
    /// subnets to receive.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let server_list = os.list_subnets().await.expect("Unable to fetch subnets");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub async fn list_subnets(&self) -> Result<Vec<Subnet>> {
        self.find_subnets().all().await
    }

    /// Prepare a new object for creation.
    ///
    /// This call returns a `NewObject` object, which is a builder
    /// to create object in object storage.
    #[cfg(feature = "object-storage")]
    pub fn new_object<C, O, B>(&self, container: C, object: O, body: B) -> NewObject<B>
    where
        C: Into<ContainerRef>,
        O: Into<String>,
        B: AsyncRead + Sync + Send + 'static,
    {
        NewObject::new(self.session.clone(), container.into(), object.into(), body)
    }

    /// Prepare a new floating IP for creation.
    ///
    /// This call returns a `NewFloatingIp` object, which is a builder
    /// to populate floating IP fields.
    #[cfg(feature = "network")]
    pub fn new_floating_ip<N>(&self, floating_network: N) -> NewFloatingIp
    where
        N: Into<NetworkRef>,
    {
        NewFloatingIp::new(self.session.clone(), floating_network.into())
    }

    /// Prepare a new key pair for creation.
    ///
    /// This call returns a `NewKeyPair` object, which is a builder to populate
    /// key pair fields.
    #[cfg(feature = "compute")]
    pub fn new_keypair<S>(&self, name: S) -> NewKeyPair
    where
        S: Into<String>,
    {
        NewKeyPair::new(self.session.clone(), name.into())
    }

    /// Prepare a new network for creation.
    ///
    /// This call returns a `NewNetwork` object, which is a builder to populate
    /// network fields.
    #[cfg(feature = "network")]
    pub fn new_network(&self) -> NewNetwork {
        NewNetwork::new(self.session.clone())
    }

    /// Prepare a new port for creation.
    ///
    /// This call returns a `NewPort` object, which is a builder to populate
    /// port fields.
    #[cfg(feature = "network")]
    pub fn new_port<N>(&self, network: N) -> NewPort
    where
        N: Into<NetworkRef>,
    {
        NewPort::new(self.session.clone(), network.into())
    }

    /// Prepare a new router for creation.
    ///
    /// This call returns a `NewRouter` object, which is a builder to populate
    /// router fields.
    #[cfg(feature = "network")]
    pub fn new_router(&self) -> NewRouter {
        NewRouter::new(self.session.clone())
    }

    /// Prepare a new server for creation.
    ///
    /// This call returns a `NewServer` object, which is a builder to populate
    /// server fields.
    #[cfg(feature = "compute")]
    pub fn new_server<S, F>(&self, name: S, flavor: F) -> NewServer
    where
        S: Into<String>,
        F: Into<FlavorRef>,
    {
        NewServer::new(self.session.clone(), name.into(), flavor.into())
    }

    /// Prepare a new subnet for creation.
    ///
    /// This call returns a `NewSubnet` object, which is a builder to populate
    /// subnet fields.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// extern crate ipnet;
    /// extern crate openstack;
    /// use std::net;
    ///
    /// # async fn async_wrapper() {
    /// let os = openstack::Cloud::from_env().await.expect("Unable to authenticate");
    /// let cidr = ipnet::Ipv4Net::new(net::Ipv4Addr::new(192, 168, 1, 0), 24)
    ///     .unwrap().into();
    /// let new_subnet = os.new_subnet("private-net", cidr)
    ///     .with_name("private-subnet")
    ///     .create().await.expect("Unable to create subnet");
    /// # }
    /// ```
    #[cfg(feature = "network")]
    pub fn new_subnet<N>(&self, network: N, cidr: ipnet::IpNet) -> NewSubnet
    where
        N: Into<NetworkRef>,
    {
        NewSubnet::new(self.session.clone(), network.into(), cidr)
    }
}

impl From<Session> for Cloud {
    fn from(value: Session) -> Cloud {
        Cloud { session: value }
    }
}
