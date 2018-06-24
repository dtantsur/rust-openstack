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

//! Floating IP support.

use std::fmt::Debug;
use std::net;
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, ErrorKind, Result, Sort};
use super::super::common::{DeletionWaiter, ListResources, NetworkRef,
                           PortRef, Refresh, ResourceId, ResourceIterator,
                           RouterRef};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::{protocol, Network, Port};


/// Structure representing a single floating IP.
#[derive(Clone, Debug)]
pub struct FloatingIp {
    session: Rc<Session>,
    inner: protocol::FloatingIp
}

/// A query to floating IP list.
#[derive(Clone, Debug)]
pub struct FloatingIpQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}


impl FloatingIp {
    /// Create a new floating IP object.
    pub(crate) fn new(session: Rc<Session>, inner: protocol::FloatingIp) -> FloatingIp {
        FloatingIp {
            session: session,
            inner: inner
        }
    }

    /// Load a FloatingIp object.
    pub(crate) fn load<Id: AsRef<str>>(session: Rc<Session>, id: Id)
            -> Result<FloatingIp> {
        let inner = session.get_floating_ip(id)?;
        Ok(FloatingIp::new(session, inner))
    }

    transparent_property! {
        #[doc = "Creation data and time (if available)."]
        created_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        #[doc = "Floating IP description."]
        description: ref Option<String>
    }

    transparent_property! {
        #[doc = "DNS domain for the floating IP (if available)."]
        dns_domain: ref Option<String>
    }

    transparent_property! {
        #[doc = "DNS domain for the floating IP (if available)."]
        dns_name: ref Option<String>
    }

    transparent_property! {
        #[doc = "IP address of the port associated with the IP (if any)."]
        fixed_ip_address: Option<net::IpAddr>
    }

    transparent_property! {
        #[doc = "Floating IP address"]
        floating_ip_address: net::IpAddr
    }

    transparent_property! {
        #[doc = "ID of the network this floating IP belongs to."]
        floating_network_id: ref String
    }

    /// Get network this floating IP belongs to.
    pub fn floating_network(&self) -> Result<Network> {
        Network::new(self.session.clone(), &self.inner.floating_network_id)
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    /// Whether the floating IP is associated.
    pub fn is_associated(&self) -> bool {
        self.inner.port_id.is_some()
    }

    transparent_property! {
        #[doc = "List of port forwardings (if any)."]
        port_forwardings: ref Vec<protocol::PortForwarding>
    }

    transparent_property! {
        #[doc = "ID of the port this IP is attached to (if any)."]
        port_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "ID of the router of this floating IP."]
        router_id: ref Option<String>
    }

    /// Fetch the port this IP is associated with.
    ///
    /// Fails with `ResourceNotFound` if the floating IP is not associated.
    pub fn port(&self) -> Result<Port> {
        match self.inner.port_id {
            Some(ref port_id) => Port::load(self.session.clone(), &port_id),
            None => Err(Error::new(ErrorKind::ResourceNotFound,
                                   "Floating IP is not associated"))
        }
    }

    transparent_property! {
        #[doc = "Status of the floating IP."]
        status: protocol::FloatingIpStatus
    }

    transparent_property! {
        #[doc = "Last update data and time (if available)."]
        updated_at: Option<DateTime<FixedOffset>>
    }

    /// Delete the floating IP.
    pub fn delete(self) -> Result<DeletionWaiter<FloatingIp>> {
        self.session.delete_floating_ip(&self.inner.id)?;
        Ok(DeletionWaiter::new(self, Duration::new(60, 0), Duration::new(1, 0)))
    }
}

impl Refresh for FloatingIp {
    /// Refresh the floating_ip.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_floating_ip(&self.inner.id)?;
        Ok(())
    }
}

impl FloatingIpQuery {
    pub(crate) fn new(session: Rc<Session>) -> FloatingIpQuery {
        FloatingIpQuery {
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
    pub fn sort_by(mut self, sort: Sort<protocol::FloatingIpSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.query.push_str("sort_key", field);
        self.query.push("sort_dir", direction);
        self
    }

    query_filter! {
        #[doc = "Filter by description."]
        set_description, with_description -> description
    }

    query_filter! {
        #[doc = "Filter by fixed IP address."]
        set_fixed_ip_address, with_fixed_ip_address -> fixed_ip_address: net::IpAddr
    }

    query_filter! {
        #[doc = "Filter by floating IP address."]
        set_floating_ip_address, with_floating_ip_address -> floating_ip_address: net::IpAddr
    }

    /// Filter by network.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn set_floating_network<N: Into<NetworkRef>>(&mut self, value: N) {
        self.query.push_str("floating_network_id", value.into());
    }

    /// Filter by network.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn with_floating_network<N: Into<NetworkRef>>(mut self, value: N) -> Self {
        self.set_floating_network(value);
        self
    }

    /// Filter by port.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn set_port<N: Into<PortRef>>(&mut self, value: N) {
        self.query.push_str("port_id", value.into());
    }

    /// Filter by network.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn with_port<N: Into<PortRef>>(mut self, value: N) -> Self {
        self.set_port(value);
        self
    }

    /// Filter by router.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn set_router<N: Into<RouterRef>>(&mut self, value: N) {
        self.query.push_str("router_id", value.into());
    }

    /// Filter by network.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn with_router<N: Into<RouterRef>>(mut self, value: N) -> Self {
        self.set_router(value);
        self
    }

    query_filter! {
        #[doc = "Filter by status."]
        set_status, with_status -> status: protocol::FloatingIpStatus
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<FloatingIp> {
        debug!("Fetching floating_ips with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<FloatingIp>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<FloatingIp> {
        debug!("Fetching one floating IP with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl ResourceId for FloatingIp {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for FloatingIp {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<FloatingIp>> {
        Ok(session.list_floating_ips(&query)?.into_iter()
           .map(|item| FloatingIp::new(session.clone(), item)).collect())
    }
}

impl IntoFallibleIterator for FloatingIpQuery {
    type Item = FloatingIp;

    type Error = Error;

    type IntoIter = ResourceIterator<FloatingIp>;

    fn into_fallible_iterator(self) -> ResourceIterator<FloatingIp> {
        self.into_iter()
    }
}
