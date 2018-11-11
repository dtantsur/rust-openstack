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

use std::collections::HashSet;
use std::fmt::Debug;
use std::net;
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;
use serde_json;

use super::super::{Error, ErrorKind, Result, Sort};
use super::super::common::{DeletionWaiter, IntoVerified, ListResources,
                           NetworkRef, PortRef, Refresh, ResourceId,
                           ResourceIterator, RouterRef, SubnetRef};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::{protocol, Network, Port};


/// Structure representing a single floating IP.
#[derive(Clone, Debug)]
pub struct FloatingIp {
    session: Rc<Session>,
    inner: protocol::FloatingIp,
    dirty: HashSet<&'static str>,
}

/// A query to floating IP list.
#[derive(Clone, Debug)]
pub struct FloatingIpQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// A request to create a floating IP.
#[derive(Clone, Debug)]
pub struct NewFloatingIp {
    session: Rc<Session>,
    inner: protocol::FloatingIp,
    floating_network: NetworkRef,
    port: Option<PortRef>,
    subnet: Option<SubnetRef>,
}


impl FloatingIp {
    /// Create a new floating IP object.
    pub(crate) fn new(session: Rc<Session>, inner: protocol::FloatingIp) -> FloatingIp {
        FloatingIp {
            session: session,
            inner: inner,
            dirty: HashSet::new(),
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

    update_field! {
        #[doc = "Update the description."]
        set_description, with_description -> description: optional String
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

    update_field! {
        #[doc = "Update which fixed IP address is associated with the floating IP."]
        set_fixed_ip_address, with_fixed_ip_address ->fixed_ip_address: optional net::IpAddr
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
        Network::load(self.session.clone(), &self.inner.floating_network_id)
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

    /// Associate this floating IP with a port.
    ///
    /// Optionally provide a fixed IP address to associate with, in case
    /// the port has several fixed IP addresses.
    ///
    /// # Warning
    ///
    /// Any changes to `fixed_ip_address` are reset on this call.
    pub fn associate<P>(&mut self, port: P, fixed_ip_address: Option<net::IpAddr>)
            -> Result<()> where P: Into<PortRef> {
        let new_port = port.into().into_verified(&self.session)?.into();
        self.update_port(new_port, fixed_ip_address)
    }

    /// Dissociate this floating IP from a port.
    ///
    /// # Warning
    ///
    /// Any changes to `fixed_ip_address` are reset on this call.
    pub fn dissociate(&mut self) -> Result<()> {
        self.update_port(serde_json::Value::Null, None)
    }

    /// Delete the floating IP.
    pub fn delete(self) -> Result<DeletionWaiter<FloatingIp>> {
        self.session.delete_floating_ip(&self.inner.id)?;
        Ok(DeletionWaiter::new(self, Duration::new(60, 0), Duration::new(1, 0)))
    }

    /// Save the changes to the floating IP.
    pub fn save(&mut self) -> Result<()> {
        let mut update = protocol::FloatingIpUpdate::default();
        save_option_fields! {
            self -> update: description fixed_ip_address
        };
        self.inner = self.session.update_floating_ip(self.id(), update)?;
        self.dirty.clear();
        Ok(())
    }

    fn update_port(&mut self, value: serde_json::Value,
                   fixed_ip_address: Option<net::IpAddr>) -> Result<()> {
        let update = protocol::FloatingIpUpdate {
            description: None,
            fixed_ip_address: fixed_ip_address,
            port_id: Some(value),
        };
        let mut inner = self.session.update_floating_ip(self.id(), update)?;

        // NOTE(dtantsur): description is independent of port.
        let desc_changed = self.dirty.contains("description");
        self.dirty.clear();
        if desc_changed {
            inner.description = self.inner.description.take();
            let _ = self.dirty.insert("description");
        }

        self.inner = inner;
        Ok(())
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

impl NewFloatingIp {
    /// Start creating a floating IP.
    pub(crate) fn new(session: Rc<Session>, floating_network: NetworkRef)
            -> NewFloatingIp {
        NewFloatingIp {
            session: session,
            inner: protocol::FloatingIp {
                created_at: None,
                description: None,
                dns_domain: None,
                dns_name: None,
                fixed_ip_address: None,
                // 0.0.0.0 is skipped when serializing
                floating_ip_address: net::IpAddr::V4(net::Ipv4Addr::new(0, 0, 0, 0)),
                // Will be replaced in create()
                floating_network_id: String::new(),
                // Dummy value, not used when serializing
                id: String::new(),
                port_id: None,
                port_forwardings: Vec::new(),
                router_id: None,
                // Dummy value, not used when serializing
                status: protocol::FloatingIpStatus::Active,
                subnet_id: None,
                updated_at: None,
            },
            floating_network: floating_network,
            port: None,
            subnet: None,
        }
    }

    /// Request creation of the port.
    pub fn create(mut self) -> Result<FloatingIp> {
        self.inner.floating_network_id = self.floating_network.into_verified(
            &self.session)?.into();
        if let Some(port) = self.port {
            self.inner.port_id = Some(port.into_verified(&self.session)?.into());
        }
        if let Some(subnet) = self.subnet {
            self.inner.subnet_id = Some(subnet.into_verified(&self.session)?.into());
        }

        let floating_ip = self.session.create_floating_ip(self.inner)?;
        Ok(FloatingIp::new(self.session, floating_ip))
    }

    creation_inner_field! {
        #[doc = "Set description of the floating IP."]
        set_description, with_description -> description: optional String
    }

    creation_inner_field! {
        #[doc = "Set DNS domain for the floating IP."]
        set_dns_domain, with_dns_domain -> dns_domain: optional String
    }

    creation_inner_field! {
        #[doc = "Set DNS name for the floating IP."]
        set_dns_name, with_dns_name -> dns_name: optional String
    }

    creation_inner_field! {
        #[doc = "Set the requested fixed IP address (required if the port has several)."]
        set_fixed_ip_address, with_fixed_ip_address -> fixed_ip_address: optional net::IpAddr
    }

    creation_inner_field! {
        #[doc = "Set the requested floating IP address."]
        set_floating_ip_address, with_floating_ip_address -> floating_ip_address: net::IpAddr
    }

    /// Set the port to associate with the new IP.
    pub fn set_port<P>(&mut self, port: P) where P: Into<PortRef> {
        self.port = Some(port.into());
    }

    /// Set the port to associate with the new IP.
    pub fn with_port<P>(mut self, port: P) -> NewFloatingIp where P: Into<PortRef> {
        self.set_port(port);
        self
    }

    /// Set the subnet to create the IP address from.
    pub fn set_subnet<P>(&mut self, subnet: P) where P: Into<SubnetRef> {
        self.subnet = Some(subnet.into());
    }

    /// Set the subnet to create the IP address from.
    pub fn with_subnet<P>(mut self, subnet: P) -> NewFloatingIp where P: Into<SubnetRef> {
        self.set_subnet(subnet);
        self
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
