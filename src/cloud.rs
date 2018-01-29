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
#[derive(Debug, Clone)]
pub struct Cloud {
    session: Session
}

impl Cloud {
    /// Create a new cloud object with a given authentication plugin.
    pub fn new<Auth: AuthMethod + 'static>(auth_method: Auth) -> Cloud {
        Cloud::new_with_session(Session::new(auth_method))
    }

    /// Create a new cloud object with a given session.
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

    /// Session object.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Extract the session object.
    pub fn into_session(self) -> Session {
        self.session
    }

    /// Find a server by its ID.
    #[cfg(feature = "compute")]
    pub fn get_server_by_id<Id: AsRef<str>>(&self, id: Id) -> ApiResult<Server> {
        Server::new(&self.session, id)
    }

    /// Build a query against server list.
    #[cfg(feature = "compute")]
    pub fn find_servers(&self) -> ServerQuery {
        ServerQuery::new(&self.session)
    }

    /// List all servers with an optional limit.
    #[cfg(feature = "compute")]
    pub fn list_servers(&self) -> ApiResult<Vec<ServerSummary>> {
        // TODO(dtantsur): pagination
        self.find_servers().fetch()
    }
}
