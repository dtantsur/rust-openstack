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

#[allow(unused_imports)]
use fallible_iterator::FallibleIterator;

#[allow(unused_imports)]
use super::{Error, ErrorKind, Result};
use super::auth::AuthMethod;
#[allow(unused_imports)]
use super::common::FlavorRef;
#[cfg(feature = "compute")]
use super::compute::{Flavor, FlavorQuery, FlavorSummary,
                     NewServer, Server, ServerQuery, ServerSummary};
#[cfg(feature = "image")]
use super::image::{Image, ImageQuery};
#[cfg(feature = "network")]
use super::network::{Network, NetworkQuery};
use super::session::Session;
#[allow(unused_imports)]
use super::utils::ResultExt;


/// OpenStack cloud API.
///
/// Provides high-level API for working with OpenStack clouds.
#[derive(Debug, Clone)]
pub struct Cloud {
    session: Session
}

impl Cloud {
    /// Create a new cloud object with a given authentication plugin.
    ///
    /// See (auth module)[auth/index.html) for details on how to authenticate
    /// against OpenStack clouds.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// ```
    pub fn new<Auth: AuthMethod + 'static>(auth_method: Auth) -> Cloud {
        Cloud {
            session: Session::new(auth_method)
        }
    }

    /// Convert this cloud into one using the given endpoint interface.
    pub fn with_endpoint_interface<S>(self, endpoint_interface: S)
            -> Cloud where S: Into<String> {
        Cloud {
            session: self.session.with_endpoint_interface(endpoint_interface)
        }
    }

    /// Refresh this `Cloud` object (renew token, refetch service catalog, etc).
    pub fn refresh(&mut self) -> Result<()> {
        self.session.auth_method_mut().refresh()
    }

    /// `Session` used with this `Cloud` object.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Build a query against flavor list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "compute")]
    pub fn find_flavors(&self) -> FlavorQuery {
        FlavorQuery::new(&self.session)
    }

    /// Build a query against image list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "image")]
    pub fn find_images(&self) -> ImageQuery {
        ImageQuery::new(&self.session)
    }

    /// Build a query against network list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query.
    #[cfg(feature = "network")]
    pub fn find_networks(&self) -> NetworkQuery {
        NetworkQuery::new(&self.session)
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
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let sorting = openstack::compute::ServerSortKey::AccessIpv4;
    /// let server_list = os.find_servers()
    ///     .sort_by(openstack::Sort::Asc(sorting)).with_limit(5)
    ///     .all().expect("Unable to fetch servers");
    /// ```
    #[cfg(feature = "compute")]
    pub fn find_servers(&self) -> ServerQuery {
        ServerQuery::new(&self.session)
    }

    /// Find a flavor by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server = os.get_flavor("m1.medium").expect("Unable to get a flavor");
    /// ```
    #[cfg(feature = "compute")]
    pub fn get_flavor<Id: Into<String>>(&self, id_or_name: Id) -> Result<Flavor> {
        let s = id_or_name.into();
        Flavor::new(&self.session, &s).if_not_found_then(|| {
            self.find_flavors().into_iter()
                .filter(|item| item.name() == &s).take(2)
                .collect::<Vec<FlavorSummary>>().and_then(|mut items| {
                    if items.len() > 1 {
                        Err(Error::new(ErrorKind::TooManyItems,
                                       "Too many flavors with this name"))
                    } else {
                        match items.pop() {
                            Some(item) => item.details(),
                            None => Err(Error::new(
                                ErrorKind::ResourceNotFound,
                                "No flavors with this name or ID"))
                        }
                    }
                })
        })
    }

    /// Find an image by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server = os.get_image("centos7").expect("Unable to get a image");
    /// ```
    #[cfg(feature = "image")]
    pub fn get_image<Id: Into<String>>(&self, id_or_name: Id) -> Result<Image> {
        let s = id_or_name.into();
        Image::new(&self.session, &s).if_not_found_then(|| {
            self.find_images().with_name(s).one()
        })
    }

    /// Find an network by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server = os.get_network("centos7").expect("Unable to get a network");
    /// ```
    #[cfg(feature = "network")]
    pub fn get_network<Id: Into<String>>(&self, id_or_name: Id) -> Result<Network> {
        let s = id_or_name.into();
        Network::new(&self.session, &s).if_not_found_then(|| {
            self.find_networks().with_name(s).one()
        })
    }

    /// Find a server by its name or ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server = os.get_server("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
    ///     .expect("Unable to get a server");
    /// ```
    #[cfg(feature = "compute")]
    pub fn get_server<Id: Into<String>>(&self, id_or_name: Id) -> Result<Server> {
        let s = id_or_name.into();
        Server::new(&self.session, &s).if_not_found_then(|| {
            self.find_servers().with_name(s.clone()).into_iter()
                .filter(|srv| srv.name() == &s).take(2)
                .collect::<Vec<ServerSummary>>().and_then(|mut srvs| {
                    if srvs.len() > 1 {
                        Err(Error::new(ErrorKind::TooManyItems,
                                       "Too many servers with this name"))
                    } else {
                        match srvs.pop() {
                            Some(srv) => srv.details(),
                            None => Err(Error::new(
                                ErrorKind::ResourceNotFound,
                                "No servers with this name or ID"))
                        }
                    }
                })
        })
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
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
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
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server_list = os.list_images().expect("Unable to fetch images");
    /// ```
    #[cfg(feature = "image")]
    pub fn list_images(&self) -> Result<Vec<Image>> {
        self.find_images().all()
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
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server_list = os.list_networks().expect("Unable to fetch networks");
    /// ```
    #[cfg(feature = "network")]
    pub fn list_networks(&self) -> Result<Vec<Network>> {
        self.find_networks().all()
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
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server_list = os.list_servers().expect("Unable to fetch servers");
    /// ```
    #[cfg(feature = "compute")]
    pub fn list_servers(&self) -> Result<Vec<ServerSummary>> {
        self.find_servers().all()
    }

    /// Prepare a new server for creation.
    ///
    /// This call returns a `NewServer` object, which is a builder to populate
    /// server fields.
    #[cfg(feature = "compute")]
    pub fn new_server<S, F>(&self, name: S, flavor: F) -> NewServer
            where S: Into<String>, F: Into<FlavorRef> {
        NewServer::new(&self.session, name.into(), flavor.into())
    }
}


impl From<Session> for Cloud {
    fn from(value: Session) -> Cloud {
        Cloud {
            session: value
        }
    }
}

impl From<Cloud> for Session {
    fn from(value: Cloud) -> Session {
        value.session
    }
}
