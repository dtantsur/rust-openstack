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
//! ```rust,no_run
//! use openstack;
//! use openstack::compute;
//!
//! let auth = openstack::auth::Identity::from_env()
//!     .expect("Unable to authenticate");
//! let session = openstack::Session::new(auth);
//! let server_manager = compute::servers::manager(&session);
//!
//! let server_list = server_manager.list().expect("Unable to fetch servers");
//! ```

use super::super::{ApiResult, Session};
use super::super::auth::Method as AuthMethod;
use super::super::service::{IntoId, ServiceApi};
use super::api::ComputeV2;
use super::protocol;

/// Structure represending filters for listing servers.
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone)]
pub struct ServerFilters {}

/// Server manager: working with virtual servers.
#[derive(Debug)]
pub struct ServerManager<'a, Auth: AuthMethod + 'a> {
    api: ComputeV2<'a, Auth>
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct Server<'a, Auth: AuthMethod + 'a> {
    manager: &'a ServerManager<'a, Auth>,
    inner: protocol::Server
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct ServerSummary<'a, Auth: AuthMethod + 'a> {
    manager: &'a ServerManager<'a, Auth>,
    inner: protocol::ServerSummary
}

/// List of servers.
pub type ServerList<'a, Auth> = Vec<ServerSummary<'a, Auth>>;

/// Constructor for server manager.
pub fn manager<'a, Auth: AuthMethod + 'a>(session: &'a Session<Auth>)
        -> ServerManager<'a, Auth> {
    ServerManager {
        api: ServiceApi::new(session)
    }
}

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
        self.manager.get(self.id())
    }
}

impl<'a, Auth: AuthMethod + 'a> ServerManager<'a, Auth> {
    /// List all servers without any filtering.
    pub fn list(&'a self) -> ApiResult<ServerList<'a, Auth>> {
        let inner: protocol::ServersRoot = try!(self.api.list("servers"));
        Ok(inner.servers.iter().map(|x| ServerSummary {
            manager: self,
            inner: x.clone()
        }).collect())
    }

    /// Get a server.
    pub fn get<Id: IntoId>(&'a self, id: Id) -> ApiResult<Server<'a, Auth>> {
        let inner: protocol::ServerRoot = try!(self.api.get("servers", id));
        Ok(Server {
            manager: self,
            inner: inner.server
        })
    }
}

#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use hyper;

    use super::super::super::auth::{NoAuth, SimpleToken};
    use super::super::super::session::test;
    use super::manager;

    // Copied from compute API reference.
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

    mock_connector!(MockServers {
        "http://127.0.2.1" => String::from("HTTP/1.1 200 OK\r\n\
                                           Server: Mock.Mock\r\n\
                                           \r\n") + SERVERS_RESPONSE
    });

    #[test]
    fn test_servers_list() {
        let auth = NoAuth::new("http://127.0.2.1/v2.1").unwrap();
        let cli = hyper::Client::with_connector(MockServers::default());
        let token = SimpleToken(String::from("abcdef"));
        let session = test::new_with_params(auth, cli, token, None);

        let mgr = manager(&session);
        let srvs = mgr.list().unwrap();
        assert_eq!(srvs.len(), 1);
        assert_eq!(srvs[0].id(),
                   "22c91117-08de-4894-9aa9-6ef382400985");
        assert_eq!(srvs[0].name(), "new-server-test");
    }
}
