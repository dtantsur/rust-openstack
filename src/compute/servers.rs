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
use std::fmt::Debug;
use std::net::{Ipv4Addr, Ipv6Addr};

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, ErrorKind, Result, Sort};
use super::super::service::{ListResources, ResourceId, ResourceIterator};
use super::super::session::Session;
use super::super::utils::Query;
use super::v2::{V2API, protocol};


/// A query to server list.
#[derive(Clone, Debug)]
pub struct ServerQuery<'session> {
    session: &'session Session,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a summary of a single server.
#[derive(Clone, Debug)]
pub struct Server<'session> {
    session: &'session Session,
    inner: protocol::Server
}

/// Structure representing a summary of a single server.
#[derive(Clone, Debug)]
pub struct ServerSummary<'session> {
    session: &'session Session,
    inner: protocol::ServerSummary
}


impl<'session> Server<'session> {
    /// Load a Server object.
    pub(crate) fn new<Id: AsRef<str>>(session: &'session Session, id: Id)
            -> Result<Server<'session>> {
        let inner = session.get_server(id)?;
        Ok(Server {
            session: session,
            inner: inner
        })
    }

    /// Get a reference to IPv4 address.
    pub fn access_ipv4(&self) -> &Option<Ipv4Addr> {
        &self.inner.accessIPv4
    }

    /// Get a reference to IPv6 address.
    pub fn access_ipv6(&self) -> &Option<Ipv6Addr> {
        &self.inner.accessIPv6
    }

    /// Get a reference to associated addresses.
    pub fn addresses(&self) -> &HashMap<String, Vec<protocol::ServerAddress>> {
        &self.inner.addresses
    }

    /// Get a reference to the availability zone.
    pub fn availability_zone(&self) -> &String {
        &self.inner.availability_zone
    }

    /// Get a reference to creation date and time.
    pub fn created_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.created
    }

    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to the image.
    pub fn image_id(&self) -> Option<&String> {
        match self.inner.image {
            Some(ref image) => Some(&image.id),
            None => None
        }
    }

    /// Whether the server has an image.
    ///
    /// May return `false` if the server was created from a volume.
    pub fn has_image(&self) -> bool {
        self.inner.image.is_some()
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get server status.
    pub fn status(&self) -> protocol::ServerStatus {
        self.inner.status
    }

    /// Get a reference to last update date and time.
    pub fn updated_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.updated
    }
}

impl<'session> ServerSummary<'session> {
    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(&self) -> Result<Server<'session>> {
        Server::new(self.session, &self.inner.id)
    }
}

impl<'session> ServerQuery<'session> {
    pub(crate) fn new(session: &'session Session) -> ServerQuery<'session> {
        ServerQuery {
            session: session,
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

    /// Filter by IPv4 address that should be used to access the server.
    pub fn with_access_ip_v4(mut self, value: Ipv4Addr) -> Self {
        self.query.push("access_ip_v4", value);
        self
    }

    /// Filter by IPv6 address that should be used to access the server.
    pub fn with_access_ip_v6(mut self, value: Ipv6Addr) -> Self {
        self.query.push("access_ipv6", value);
        self
    }

    /// Filter by availability zone.
    pub fn with_availability_zone<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("availability_zone", value);
        self
    }

    /// Filter by flavor.
    pub fn with_flavor<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("flavor", value);
        self
    }

    /// Filter by host name.
    pub fn with_hostname<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("hostname", value);
        self
    }

    /// Filter by image ID.
    pub fn with_image<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("image", value);
        self
    }

    /// Filter by an IPv4 address.
    pub fn with_ip_v4(mut self, value: Ipv4Addr) -> Self {
        self.query.push("ip", value);
        self
    }

    /// Filter by an IPv6 address.
    pub fn with_ip_v6(mut self, value: Ipv6Addr) -> Self {
        self.query.push("ip6", value);
        self
    }

    /// Filter by server name (a database regular expression).
    pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("name", value);
        self
    }

    /// Filter by power state.
    pub fn with_power_state<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("power_state", value);
        self
    }

    /// Filter by project ID (also commonly known as tenant ID).
    pub fn with_project_id<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("project_id", value);
        self
    }

    /// Filter by server status.
    pub fn with_status<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("status", value);
        self
    }

    /// Filter by user ID.
    pub fn with_user_id<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("user_id", value);
        self
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// This iterator yields only `ServerSummary` objects, containing
    /// IDs and names. Use `into_iter_detailed` for full `Server` objects.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<'session, ServerSummary<'session>> {
        ResourceIterator::new(self.session, self.query)
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// This iterator yields full `Server` objects. If you only need IDs
    /// and/or names, use `into_iter` to save bandwidth.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter_detailed(self) -> ResourceIterator<'session, Server<'session>> {
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<ServerSummary<'session>>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<ServerSummary<'session>> {
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        let mut iter = self.into_iter();
        match iter.next()? {
            Some(result) => if iter.next()?.is_some() {
                Err(Error::new(ErrorKind::TooManyItems,
                               "Query returned more than one result"))
            } else {
                Ok(result)
            },
            None => Err(Error::new(ErrorKind::ResourceNotFound,
                                   "Query returned no results"))
        }
    }
}

impl<'session> ResourceId for ServerSummary<'session> {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl<'session> ListResources<'session> for ServerSummary<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<ServerSummary<'session>>> {
        Ok(session.list_servers(&query)?.into_iter().map(|srv| ServerSummary {
            session: session,
            inner: srv
        }).collect())
    }
}

impl<'session> ResourceId for Server<'session> {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl<'session> ListResources<'session> for Server<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<Server<'session>>> {
        Ok(session.list_servers_detail(&query)?.into_iter().map(|srv| Server {
            session: session,
            inner: srv
        }).collect())
    }
}

impl<'session> IntoFallibleIterator for ServerQuery<'session> {
    type Item = ServerSummary<'session>;

    type Error = Error;

    type IntoIter = ResourceIterator<'session, ServerSummary<'session>>;

    fn into_fallible_iterator(self) -> ResourceIterator<'session, ServerSummary<'session>> {
        self.into_iter()
    }
}
