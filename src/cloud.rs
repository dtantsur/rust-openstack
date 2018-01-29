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
use super::ApiResult;
use super::auth::AuthMethod;
#[cfg(feature = "compute")]
use super::compute::{Server, ServerQuery, ServerSummary};
use super::session::Session;


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
        Cloud::new_with_session(Session::new(auth_method))
    }

    /// Create a new cloud object with a given session.
    ///
    /// This constructor can be used to modify `Session` parameters before
    /// using it in the `Cloud` object. This is an advanced feature and
    /// should generally be avoided.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let session = openstack::session::Session::new(auth);
    /// let os = openstack::Cloud::new_with_session(session);
    pub fn new_with_session(session: Session) -> Cloud {
        Cloud {
            session: session
        }
    }

    /// Convert this cloud into one using the given endpoint interface.
    pub fn with_endpoint_interface<S>(self, endpoint_interface: S)
            -> Cloud where S: Into<String> {
        Cloud {
            session: self.session.with_endpoint_interface(endpoint_interface)
        }
    }

    /// `Session` used with this `Cloud` object.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Extract the `Session` object, destroying this `Cloud`.
    pub fn into_session(self) -> Session {
        self.session
    }

    /// Find a server by its ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openstack;
    ///
    /// let auth = openstack::auth::from_env().expect("Unable to authenticate");
    /// let os = openstack::Cloud::new(auth);
    /// let server = os.get_server_by_id("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
    ///     .expect("Unable to get a server");
    /// ```
    #[cfg(feature = "compute")]
    pub fn get_server_by_id<Id: AsRef<str>>(&self, id: Id) -> ApiResult<Server> {
        Server::new(&self.session, id)
    }

    /// Build a query against server list.
    ///
    /// The returned object is a builder that should be used to construct
    /// the query. The results can be received with a
    /// [fetch](compute/struct.ServerQuery.html#method.fetch) call.
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
    ///     .fetch().expect("Unable to fetch servers");
    /// ```
    #[cfg(feature = "compute")]
    pub fn find_servers(&self) -> ServerQuery {
        ServerQuery::new(&self.session)
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
    pub fn list_servers(&self) -> ApiResult<Vec<ServerSummary>> {
        // TODO(dtantsur): pagination
        self.find_servers().fetch()
    }
}
