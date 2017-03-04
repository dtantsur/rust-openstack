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

use super::super::{ApiError, Session};
use super::super::auth::AuthMethod;
use super::super::session::ServiceApi;
use super::super::utils::IntoId;
use super::protocol::{self, ComputeApiV2};

/// Structure represending filters for listing servers.
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone)]
pub struct ServerFilters {}

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

/// Server manager: working with virtual servers.
#[derive(Debug)]
pub struct ServerManager<'a, A: AuthMethod + 'a> {
    api: ServiceApi<'a, A, ComputeApiV2>
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct Server<'a, A: AuthMethod + 'a> {
    manager: &'a ServerManager<'a, A>,
    inner: protocol::Server
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct ServerSummary<'a, A: AuthMethod + 'a> {
    manager: &'a ServerManager<'a, A>,
    inner: protocol::ServerSummary
}

/// List of servers.
pub type ServerList<'a, A> = Vec<ServerSummary<'a, A>>;

/// Constructor for server manager.
pub fn servers<'a, A: AuthMethod + 'a>(session: &'a Session<A>)
        -> ServerManager<'a, A> {
    ServerManager {
        api: ServiceApi::new(session)
    }
}

impl<'a, A: AuthMethod + 'a> Server<'a, A> {
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

impl<'a, A: AuthMethod + 'a> ServerSummary<'a, A> {
    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(self) -> Result<Server<'a, A>, ApiError> {
        self.manager.get(self.id())
    }
}

impl<'a, A: AuthMethod + 'a> ServerManager<'a, A> {
    /// List all servers without any filtering.
    pub fn list(&'a self) -> Result<ServerList<'a, A>, ApiError> {
        let inner: protocol::ServersRoot = try!(self.api.list("servers"));
        Ok(inner.servers.iter().map(|x| ServerSummary {
            manager: self,
            inner: x.clone()
        }).collect())
    }

    /// Get a server.
    pub fn get<Id: IntoId>(&'a self, id: Id)
            -> Result<Server<'a, A>, ApiError> {
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

    use super::super::super::Session;
    use super::super::super::auth::base::{NoAuth, SimpleAuthToken};
    use super::servers;

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
        let token = SimpleAuthToken(String::from("abcdef"));
        let session = Session::new_with_params(auth, cli, token);

        let api = servers(&session);
        let srvs = api.list().unwrap();
        assert_eq!(srvs.len(), 1);
        assert_eq!(srvs[0].id(),
                   "22c91117-08de-4894-9aa9-6ef382400985");
        assert_eq!(srvs[0].name(), "new-server-test");
    }
}
