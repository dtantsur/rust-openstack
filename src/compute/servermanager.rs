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
use super::api::{ComputeApi, ServerFilters};
use super::protocol;

/// Server manager: working with virtual servers.
#[derive(Debug)]
pub struct ServerManager<'a, A: AuthMethod + 'a> {
    api: ComputeApi<'a, A>
}

/// Structure representing a single server.
#[derive(Debug)]
pub struct Server<'a, A: AuthMethod + 'a> {
    manager: &'a ServerManager<'a, A>,
    inner: protocol::Server
}

/// List of servers.
pub type ServerList<'a, A> = Vec<Server<'a, A>>;

/// Constructor for server manager.
pub fn servers<'a, A: AuthMethod + 'a>(session: &'a Session<A>)
        -> ServerManager<'a, A> {
    ServerManager {
        api: ComputeApi::new(session)
    }
}

impl<'a, A: AuthMethod + 'a>Server<'a, A> {
    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }
}

impl<'a, A: AuthMethod + 'a> ServerManager<'a, A> {
    /// List all servers without any filtering.
    pub fn list(&'a self) -> Result<ServerList<'a, A>, ApiError> {
        let inner = try!(self.api.list_servers(ServerFilters::new()));
        Ok(inner.iter().map(|x| Server {
            manager: self,
            inner: x.clone()
        }).collect())
    }
}
