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

//! Cloud API.

use std::rc::Rc;

use super::Result;
use super::auth::{self, AuthMethod};
#[allow(unused_imports)]
use super::common::{FlavorRef, NetworkRef};
#[cfg(feature = "compute")]
use super::compute::{Flavor, FlavorQuery, FlavorSummary, KeyPair, KeyPairQuery,
                     NewKeyPair, NewServer, Server, ServerQuery, ServerSummary};
#[cfg(feature = "image")]
use super::image::{Image, ImageQuery};
#[cfg(feature = "network")]
use super::network::{Network, NetworkQuery, NewPort, Port, PortQuery,
                     Subnet, SubnetQuery};
use super::session::Session;


/// OpenStack cloud API.
///
/// Provides high-level API for working with OpenStack clouds.
#[derive(Debug, Clone)]
pub struct Cloud {
    session: Rc<Session>
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
    /// fn cloud_from_env() -> openstack::Result<openstack::Cloud> {
    ///     openstack::auth::from_env().map(openstack::Cloud::new)
    /// }
    ///
    /// # fn main() { cloud_from_env().unwrap(); }
    /// ```
    ///
    /// Note: in this particular case it's better to use
    /// [from_env](#method.from_env).
    pub fn new<Auth: AuthMethod + 'static>(auth_method: Auth) -> Cloud {
        Cloud {
            session: Rc::new(Session::new(auth_method))
        }
    }

    /// Create a new cloud object from environment variables.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # fn cloud_from_env() -> openstack::Result<()> {
    /// let os = openstack::Cloud::from_env()?;
    /// # Ok(()) }
    /// # fn main() { cloud_from_env().unwrap(); }
    /// ```
    pub fn from_env() -> Result<Cloud> {
        Ok(Cloud {
            session: Rc::new(Session::new(auth::from_env()?))
        })
    }

    /// Convert this cloud into one using the given endpoint interface.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// fn cloud_from_env() -> openstack::Result<openstack::Cloud> {
    ///     openstack::Cloud::from_env()
    ///         .map(|os| os.with_endpoint_interface("internal"))
    /// }
    ///
    /// # fn main() { cloud_from_env().unwrap(); }
    /// ```
    pub fn with_endpoint_interface<S>(mut self, endpoint_interface: S)
            -> Cloud where S: Into<String> {
        Rc::make_mut(&mut self.session).set_endpoint_interface(endpoint_interface);
        self
    }

    /// Refresh this `Cloud` object (renew token, refetch service catalog, etc).
    pub fn refresh(&mut self) -> Result<()> {
        Rc::make_mut(&mut self.session).auth_method_mut().refresh()
    }

    /// Build a query against flavor list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "compute")]
    pub fn find_flavors(&self) -> FlavorQuery {
        FlavorQuery::new(self.session.clone())
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let sorting = openstack::compute::ServerSortKey::AccessIpv4;
    /// let server_list = os.find_servers()
    ///     .sort_by(openstack::Sort::Asc(sorting)).with_limit(5)
    ///     .all().expect("Unable to fetch servers");
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

    /// Find a flavor by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_flavor("m1.medium").expect("Unable to get a flavor");
    /// ```
    #[cfg(feature = "compute")]
    pub fn get_flavor<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Flavor> {
        Flavor::new(self.session.clone(), id_or_name)
    }

    /// Find an image by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_image("centos7").expect("Unable to get a image");
    /// ```
    #[cfg(feature = "image")]
    pub fn get_image<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Image> {
        Image::new(self.session.clone(), id_or_name)
    }

    /// Find a key pair by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_keypair("default").expect("Unable to get a key pair");
    /// ```
    #[cfg(feature = "compute")]
    pub fn get_keypair<Id: AsRef<str>>(&self, name: Id) -> Result<KeyPair> {
        KeyPair::new(self.session.clone(), name)
    }

    /// Find an network by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_network("centos7").expect("Unable to get a network");
    /// ```
    #[cfg(feature = "network")]
    pub fn get_network<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Network> {
        Network::new(self.session.clone(), id_or_name)
    }

    /// Find an port by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_port("4d9c1710-fa02-49f9-8218-291024ef4140")
    ///     .expect("Unable to get a port");
    /// ```
    #[cfg(feature = "network")]
    pub fn get_port<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Port> {
        Port::load(self.session.clone(), id_or_name)
    }

    /// Find a server by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_server("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
    ///     .expect("Unable to get a server");
    /// ```
    #[cfg(feature = "compute")]
    pub fn get_server<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Server> {
        Server::new(self.session.clone(), id_or_name)
    }

    /// Find an subnet by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server = os.get_subnet("private-subnet")
    ///     .expect("Unable to get a subnet");
    /// ```
    #[cfg(feature = "network")]
    pub fn get_subnet<Id: AsRef<str>>(&self, id_or_name: Id) -> Result<Subnet> {
        Subnet::load(self.session.clone(), id_or_name)
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server_list = os.list_flavors().expect("Unable to fetch flavors");
    /// ```
    #[cfg(feature = "compute")]
    pub fn list_flavors(&self) -> Result<Vec<FlavorSummary>> {
        self.find_flavors().all()
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server_list = os.list_images().expect("Unable to fetch images");
    /// ```
    #[cfg(feature = "image")]
    pub fn list_images(&self) -> Result<Vec<Image>> {
        self.find_images().all()
    }

    /// List all key pairs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let result = os.list_keypairs().expect("Unable to fetch key pairs");
    /// ```
    #[cfg(feature = "compute")]
    pub fn list_keypairs(&self) -> Result<Vec<KeyPair>> {
        self.find_keypairs().all()
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server_list = os.list_networks().expect("Unable to fetch networks");
    /// ```
    #[cfg(feature = "network")]
    pub fn list_networks(&self) -> Result<Vec<Network>> {
        self.find_networks().all()
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server_list = os.list_ports().expect("Unable to fetch ports");
    /// ```
    #[cfg(feature = "network")]
    pub fn list_ports(&self) -> Result<Vec<Port>> {
        self.find_ports().all()
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server_list = os.list_servers().expect("Unable to fetch servers");
    /// ```
    #[cfg(feature = "compute")]
    pub fn list_servers(&self) -> Result<Vec<ServerSummary>> {
        self.find_servers().all()
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
    /// let os = openstack::Cloud::from_env().expect("Unable to authenticate");
    /// let server_list = os.list_subnets().expect("Unable to fetch subnets");
    /// ```
    #[cfg(feature = "network")]
    pub fn list_subnets(&self) -> Result<Vec<Subnet>> {
        self.find_subnets().all()
    }

    /// Prepare a new key pair for creation.
    ///
    /// This call returns a `NewKeyPair` object, which is a builder to populate
    /// key pair fields.
    #[cfg(feature = "compute")]
    pub fn new_keypair<S>(&self, name: S) -> NewKeyPair where S: Into<String> {
        NewKeyPair::new(self.session.clone(), name.into())
    }

    /// Prepare a new port for creation.
    ///
    /// This call returns a `NewPort` object, which is a builder to populate
    /// port fields.
    #[cfg(feature = "network")]
    pub fn new_port<N>(&self, network: N) -> NewPort where N: Into<NetworkRef> {
        NewPort::new(self.session.clone(), network.into())
    }

    /// Prepare a new server for creation.
    ///
    /// This call returns a `NewServer` object, which is a builder to populate
    /// server fields.
    #[cfg(feature = "compute")]
    pub fn new_server<S, F>(&self, name: S, flavor: F) -> NewServer
            where S: Into<String>, F: Into<FlavorRef> {
        NewServer::new(self.session.clone(), name.into(), flavor.into())
    }
}


impl From<Session> for Cloud {
    fn from(value: Session) -> Cloud {
        Cloud {
            session: Rc::new(value)
        }
    }
}
