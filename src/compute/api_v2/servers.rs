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
//!
//! # Examples
//!
//! Listing summaries of all servers:
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::Identity::from_env()
//!     .expect("Unable to authenticate");
//! let session = openstack::Session::new(auth);
//! let server_list = openstack::compute::v2(&session).servers().list()
//!     .expect("Unable to fetch servers");
//! ```
//! Fetching server details by its UUID:
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::Identity::from_env()
//!     .expect("Unable to authenticate");
//! let session = openstack::Session::new(auth);
//! let server = openstack::compute::v2(&session).servers()
//!     .get("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
//!     .expect("Unable to get a server");
//! println!("Server name is {}", server.name());
//! ```

use super::super::super::{ApiResult, Session};
use super::super::super::auth::Method as AuthMethod;
use super::super::super::service::ServiceWrapper;
use super::base::V2ServiceType;
use super::protocol;


type V2ServiceWrapper<'a, Auth> = ServiceWrapper<'a, Auth, V2ServiceType>;

/// Structure represending filters for listing servers.
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone)]
pub struct ServerFilters {}

/// Server manager: working with virtual servers.
#[derive(Debug)]
pub struct ServerManager<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct Server<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>,
    inner: protocol::Server
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct ServerSummary<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>,
    inner: protocol::ServerSummary
}

/// List of servers.
pub type ServerList<'a, Auth> = Vec<ServerSummary<'a, Auth>>;

impl ServerFilters {
    /// Create empty server filters.
    pub fn new() -> ServerFilters {
        ServerFilters {}
    }
}

impl Default for ServerFilters {
    fn default() -> ServerFilters {
        ServerFilters::new()
    }
}

impl<'a, Auth: AuthMethod + 'a> Server<'a, Auth> {
    /// Get a reference to IPv4 address.
    pub fn access_ipv4(&self) -> &String {
        &self.inner.accessIPv4
    }

    /// Get a reference to IPv6 address.
    pub fn access_ipv6(&self) -> &String {
        &self.inner.accessIPv6
    }

    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get server status.
    pub fn status(&self) -> &String {
        &self.inner.status
    }
}

impl<'a, Auth: AuthMethod + 'a> ServerSummary<'a, Auth> {
    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(self) -> ApiResult<Server<'a, Auth>> {
        ServerManager::get_server(self.service.clone(), &self.inner.id)
    }
}

impl<'a, Auth: AuthMethod + 'a> ServerManager<'a, Auth> {
    /// Constructor for server manager.
    pub fn new(session: &'a Session<Auth>) -> ServerManager<'a, Auth> {
        ServerManager {
            service: ServiceWrapper::new(session)
        }
    }

    /// List all servers without any filtering.
    pub fn list(&self) -> ApiResult<ServerList<'a, Auth>> {
        trace!("Listing all compute servers");
        let inner: protocol::ServersRoot = try!(
            self.service.http_get(&["servers"])
        );
        debug!("Received {} compute servers", inner.servers.len());
        trace!("Received servers: {:?}", inner.servers);
        Ok(inner.servers.iter().map(|x| ServerSummary {
            service: self.service.clone(),
            inner: x.clone()
        }).collect())
    }

    /// Get a server.
    pub fn get<Id: AsRef<str>>(&self, id: Id) -> ApiResult<Server<'a, Auth>> {
        ServerManager::get_server(self.service.clone(), id.as_ref())
    }

    fn get_server(service: V2ServiceWrapper<'a, Auth>, id: &str)
            -> ApiResult<Server<'a, Auth>> {
        trace!("Get compute server {}", id);
        let inner: protocol::ServerRoot = try!(
            service.http_get(&["servers", id])
        );
        trace!("Received {:?}", inner.server);
        Ok(Server {
            service: service,
            inner: inner.server
        })
    }
}

#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use hyper;

    use super::super::super::super::auth::{NoAuth, SimpleToken};
    use super::super::super::super::session::test;
    use super::super::base::test as api_test;
    use super::ServerManager;

    const SERVERS_RESPONSE: &'static str = r#"
    {
        "servers": [
            {
                "id": "22c91117-08de-4894-9aa9-6ef382400985",
                "links": [
                    {
                        "href": "http://openstack.example.com/v2/6f70656e737461636b20342065766572/servers/22c91117-08de-4894-9aa9-6ef382400985",
                        "rel": "self"
                    },
                    {
                        "href": "http://openstack.example.com/6f70656e737461636b20342065766572/servers/22c91117-08de-4894-9aa9-6ef382400985",
                        "rel": "bookmark"
                    }
                ],
                "name": "new-server-test"
            }
        ]
    }"#;

    mock_connector_in_order!(MockServers {
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + api_test::ONE_VERSION_RESPONSE
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + SERVERS_RESPONSE
    });

    #[test]
    fn test_servers_list() {
        let auth = NoAuth::new("http://127.0.2.1/v2.1").unwrap();
        let cli = hyper::Client::with_connector(MockServers::default());
        let token = SimpleToken(String::from("abcdef"));
        let session = test::new_with_params(auth, cli, token, None);

        let mgr = ServerManager::new(&session);
        let srvs = mgr.list().unwrap();
        assert_eq!(srvs.len(), 1);
        assert_eq!(srvs[0].id(),
                   "22c91117-08de-4894-9aa9-6ef382400985");
        assert_eq!(srvs[0].name(), "new-server-test");
    }
}
