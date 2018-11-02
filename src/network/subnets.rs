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

//! Subnets management via Network API.

use std::rc::Rc;
use std::fmt::Debug;
use std::net;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use ipnet;
use serde::Serialize;

use super::super::{Error, Result, Sort};
use super::super::common::{DeletionWaiter, ListResources, NetworkRef, SubnetRef,
                           Refresh, ResourceId, ResourceIterator};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::{protocol, Network};


/// A query to subnet list.
#[derive(Clone, Debug)]
pub struct SubnetQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a subnet - a virtual NIC.
#[derive(Clone, Debug)]
pub struct Subnet {
    session: Rc<Session>,
    inner: protocol::Subnet
}

impl Subnet {
    /// Create a subnet object.
    pub(crate) fn new(session: Rc<Session>, inner: protocol::Subnet) -> Subnet {
        Subnet {
            session: session,
            inner: inner
        }
    }

    /// Load a Subnet object.
    pub(crate) fn load<Id: AsRef<str>>(session: Rc<Session>, id: Id)
            -> Result<Subnet> {
        let inner = session.get_subnet(id)?;
        Ok(Subnet::new(session, inner))
    }

    transparent_property! {
        #[doc = "Allocation pools for DHCP."]
        allocation_pools: ref Vec<protocol::AllocationPool>
    }

    transparent_property! {
        #[doc = "Network address of this subnet."]
        cidr: ipnet::IpNet
    }

    transparent_property! {
        #[doc = "Creation data and time (if available)."]
        created_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        #[doc = "Subnet description."]
        description: ref Option<String>
    }

    transparent_property! {
        #[doc = "Whether DHCP is enabled."]
        dhcp_enabled: bool
    }

    transparent_property! {
        #[doc = "List of DNS servers."]
        dns_nameservers: ref Vec<String>
    }

    transparent_property! {
        #[doc = "Gateway IP address (if any)."]
        gateway_ip: Option<net::IpAddr>
    }

    transparent_property! {
        #[doc = "Statically configured routes."]
        host_routes: ref Vec<protocol::HostRoute>
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "IP protocol version."]
        ip_version: protocol::IpVersion
    }

    transparent_property! {
        #[doc = "Address assignment mode for IPv6."]
        ipv6_address_mode: Option<protocol::Ipv6Mode>
    }

    transparent_property! {
        #[doc = "Router advertisement mode for IPv6."]
        ipv6_router_advertisement_mode: Option<protocol::Ipv6Mode>
    }

    transparent_property! {
        #[doc = "Subnet name."]
        name: ref Option<String>
    }

    /// Get network associated with this subnet.
    pub fn network(&self) -> Result<Network> {
        Network::load(self.session.clone(), &self.inner.network_id)
    }

    transparent_property! {
        #[doc = "ID of the network this subnet belongs to."]
        network_id: ref String
    }

    transparent_property! {
        #[doc = "Last update data and time (if available)."]
        updated_at: Option<DateTime<FixedOffset>>
    }

    /// Delete the subnet.
    pub fn delete(self) -> Result<DeletionWaiter<Subnet>> {
        self.session.delete_subnet(&self.inner.id)?;
        Ok(DeletionWaiter::new(self, Duration::new(60, 0), Duration::new(1, 0)))
    }
}

impl Refresh for Subnet {
    /// Refresh the subnet.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_subnet_by_id(&self.inner.id)?;
        Ok(())
    }
}

impl SubnetQuery {
    pub(crate) fn new(session: Rc<Session>) -> SubnetQuery {
        SubnetQuery {
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
    pub fn sort_by(mut self, sort: Sort<protocol::SubnetSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.query.push_str("sort_key", field);
        self.query.push("sort_dir", direction);
        self
    }

    query_filter! {
        #[doc = "Filter by CIDR."]
        set_cidr, with_cidr -> cidr: ipnet::IpNet
    }

    query_filter! {
        #[doc = "Filter by description."]
        set_description, with_description -> description
    }

    query_filter! {
        #[doc = "Filter by whether DHCP is enabled."]
        set_dhcp_enabled, with_dhcp_enabled -> enable_dhcp: bool
    }

    query_filter! {
        #[doc = "Filter by gateway IP."]
        set_gateway_ip, with_gateway_ip -> gateway_ip: net::IpAddr
    }

    query_filter! {
        #[doc = "Filter by IPv6 address assignment mode."]
        set_ipv6_address_mode, with_ipv6_address_mode ->
            ipv6_address_mode: protocol::Ipv6Mode
    }

    query_filter! {
        #[doc = "Filter by IPv6 router advertisement mode."]
        set_ipv6_router_advertisement_mode, with_ipv6_router_advertisement ->
            ipv6_ra_mode: protocol::Ipv6Mode
    }

    query_filter! {
        #[doc = "Filter by subnet name."]
        set_name, with_name -> name
    }

    /// Filter by network.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn set_network<N: Into<NetworkRef>>(&mut self, value: N) {
        self.query.push_str("network_id", value.into());
    }

    /// Filter by network.
    ///
    /// # Warning
    ///
    /// Due to architectural limitations, names do not work here.
    pub fn with_network<N: Into<NetworkRef>>(mut self, value: N) -> Self {
        self.set_network(value);
        self
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<Subnet> {
        debug!("Fetching subnets with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Subnet>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Subnet> {
        debug!("Fetching one subnet with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl ResourceId for Subnet {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for Subnet {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<Subnet>> {
        Ok(session.list_subnets(&query)?.into_iter()
           .map(|item| Subnet::new(session.clone(), item)).collect())
    }
}

impl IntoFallibleIterator for SubnetQuery {
    type Item = Subnet;

    type Error = Error;

    type IntoIter = ResourceIterator<Subnet>;

    fn into_fallible_iterator(self) -> ResourceIterator<Subnet> {
        self.into_iter()
    }
}

impl From<Subnet> for SubnetRef {
    fn from(value: Subnet) -> SubnetRef {
        SubnetRef::new_verified(value.inner.id)
    }
}

impl SubnetRef {
    /// Verify this reference and convert to an ID, if possible.
    #[cfg(feature = "network")]
    #[allow(unused)] // TODO(dtantsur): remove when something uses this
    pub(crate) fn into_verified(self, session: &Session) -> Result<String> {
        Ok(if self.verified {
            self.value
        } else {
            session.get_subnet(&self.value)?.id
        })
    }
}
