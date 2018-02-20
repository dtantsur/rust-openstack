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
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, ErrorKind, Result, Sort, Waiter};
use super::super::adapters::{ToFlavorId, ToImageId};
use super::super::service::{ListResources, ResourceId, ResourceIterator};
use super::super::session::Session;
use super::super::utils::{self, Query};
use super::base::V2API;
use super::protocol;


/// A query to server list.
#[derive(Clone, Debug)]
pub struct ServerQuery<'session> {
    session: &'session Session,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a single server.
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

/// Waiter for server status to change.
#[derive(Debug)]
pub struct ServerStatusWaiter<'server> {
    server: &'server mut Server<'server>,
    target: protocol::ServerStatus
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

    /// Refresh the server.
    pub fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_server(&self.inner.id)?;
        Ok(())
    }

    /// Get a reference to IPv4 address.
    ///
    /// If not None, this address should be used to access the server instead
    /// of one from the `addresses` method.
    pub fn access_ipv4(&self) -> &Option<Ipv4Addr> {
        &self.inner.accessIPv4
    }

    /// Get a reference to IPv6 address.
    ///
    /// If not None, this address should be used to access the server instead
    /// of one from the `addresses` method.
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
    ///
    /// May be None if the server was created from a volume.
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

    /// Get server power state.
    pub fn power_state(&self) -> protocol::ServerPowerState {
        self.inner.power_state
    }

    /// Get server status.
    pub fn status(&self) -> protocol::ServerStatus {
        self.inner.status
    }

    /// Get a reference to last update date and time.
    pub fn updated_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.updated
    }

    /// Start the server, optionally wait for it to be active.
    pub fn start(&'session mut self) -> Result<ServerStatusWaiter<'session>> {
        self.session.server_simple_action(&self.inner.id, "os-start")?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::Active
        })
    }

    /// Stop the server, optionally wait for it to be powered off.
    pub fn stop(&'session mut self) -> Result<ServerStatusWaiter<'session>> {
        self.session.server_simple_action(&self.inner.id, "os-stop")?;
        Ok(ServerStatusWaiter {
            server: self,
            target: protocol::ServerStatus::ShutOff
        })
    }

    /// Delete the server.
    pub fn delete(self) -> Result<()> {
        // TODO(dtantsur): implement wait
        self.session.delete_server(&self.inner.id)
    }
}

impl<'server> ServerStatusWaiter<'server> {
    /// Current state of the server.
    ///
    /// Valid as of the last poll.
    pub fn current(&self) -> &Server<'server> {
        self.server
    }
}

impl<'server> Waiter<()> for ServerStatusWaiter<'server> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        // TODO(dtantsur): vary depending on target?
        Some(Duration::new(600, 0))
    }

    fn default_delay(&self) -> Duration {
        Duration::new(1, 0)
    }

    fn timeout_error_message(&self) -> String {
        format!("Timeout waiting for server {} to reach state {}",
                self.server.id(), self.target)
    }

    fn poll(&mut self) -> Result<Option<()>> {
        self.server.refresh()?;
        if self.server.status() == self.target {
            debug!("Server {} reached state {}", self.server.id(), self.target);
            Ok(Some(()))
        } else if self.server.status() == protocol::ServerStatus::Error {
            debug!("Failed to move server {} to {} - status is ERROR",
                   self.server.id(), self.target);
            Err(Error::new(ErrorKind::OperationFailed,
                           format!("Server {} got into ERROR state",
                                   self.server.id())))
        } else {
            trace!("Still waiting for server {} to get to state {}, current is {}",
                   self.server.id(), self.server.status(), self.target);
            Ok(None)
        }
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

    /// Delete the server.
    pub fn delete(self) -> Result<()> {
        // TODO(dtantsur): implement wait
        self.session.delete_server(&self.inner.id)
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
    pub fn with_flavor<T: ToFlavorId>(mut self, value: T) -> Self {
        self.query.push_str("flavor", value.to_flavor_id());
        self
    }

    /// Filter by host name.
    pub fn with_hostname<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("hostname", value);
        self
    }

    /// Filter by image ID.
    pub fn with_image<T: ToImageId>(mut self, value: T) -> Self {
        self.query.push_str("image", value.to_image_id());
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

    /// Filter by project ID (also commonly known as tenant ID).
    pub fn with_project_id<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("project_id", value);
        self
    }

    /// Filter by server status.
    pub fn with_status(mut self, value: protocol::ServerStatus) -> Self {
        self.query.push_str("status", value.to_string());
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

        utils::fetch_one(self)
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
