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

use std::collections::HashSet;
use std::net;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{FallibleIterator, IntoFallibleIterator};
use ipnet;

use super::super::common::{
    DeletionWaiter, IntoVerified, NetworkRef, Refresh, ResourceIterator, ResourceQuery, SubnetRef,
};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, Result, Sort};
use super::{api, protocol, Network};

/// A query to subnet list.
#[derive(Clone, Debug)]
pub struct SubnetQuery {
    session: Arc<Session>,
    query: Query,
    can_paginate: bool,
    network: Option<NetworkRef>,
}

/// Structure representing a subnet - a virtual NIC.
#[derive(Clone, Debug)]
pub struct Subnet {
    session: Arc<Session>,
    inner: protocol::Subnet,
    dirty: HashSet<&'static str>,
}

/// A request to create a subnet.
#[derive(Clone, Debug)]
pub struct NewSubnet {
    session: Arc<Session>,
    inner: protocol::Subnet,
    network: NetworkRef,
}

impl Subnet {
    /// Create a subnet object.
    pub(crate) fn new(session: Arc<Session>, inner: protocol::Subnet) -> Subnet {
        Subnet {
            session,
            inner,
            dirty: HashSet::new(),
        }
    }

    /// Load a Subnet object.
    pub(crate) fn load<Id: AsRef<str>>(session: Arc<Session>, id: Id) -> Result<Subnet> {
        let inner = api::get_subnet(&session, id)?;
        Ok(Subnet::new(session, inner))
    }

    transparent_property! {
        #[doc = "Allocation pools for DHCP."]
        allocation_pools: ref Vec<protocol::AllocationPool>
    }

    update_field_mut! {
        #[doc = "Update the allocation pools for DHCP."]
        allocation_pools_mut, set_allocation_pools, with_allocation_pools
            -> allocation_pools: Vec<protocol::AllocationPool>
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

    update_field! {
        #[doc = "Update the description."]
        set_description, with_description -> description: optional String
    }

    transparent_property! {
        #[doc = "Whether DHCP is enabled."]
        dhcp_enabled: bool
    }

    update_field! {
        #[doc = "Update whether DHCP is enabled."]
        set_dhcp_enabled, with_dhcp_enabled -> dhcp_enabled: bool
    }

    transparent_property! {
        #[doc = "List of DNS servers."]
        dns_nameservers: ref Vec<String>
    }

    update_field_mut! {
        #[doc = "Update the list of DNS servers."]
        dns_nameservers_mut, set_dns_nameservers, with_dns_nameservers
            -> dns_nameservers: Vec<String>
    }

    transparent_property! {
        #[doc = "Gateway IP address (if any)."]
        gateway_ip: Option<net::IpAddr>
    }

    update_field! {
        #[doc = "Update the gateway IP."]
        set_gateway_ip, with_gateway_ip -> gateway_ip: optional net::IpAddr
    }

    transparent_property! {
        #[doc = "Statically configured routes."]
        host_routes: ref Vec<protocol::HostRoute>
    }

    update_field_mut! {
        #[doc = "Update the statically configured routes."]
        host_routes_mut, set_host_routes, with_host_routes
            -> host_routes: Vec<protocol::HostRoute>
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

    update_field! {
        #[doc = "Update the name."]
        set_name, with_name -> name: optional String
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
        api::delete_subnet(&self.session, &self.inner.id)?;
        Ok(DeletionWaiter::new(
            self,
            Duration::new(60, 0),
            Duration::new(1, 0),
        ))
    }

    /// Whether the subnet is modified.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Save the changes to the subnet.
    pub fn save(&mut self) -> Result<()> {
        let mut update = protocol::SubnetUpdate::default();
        save_fields! {
            self -> update: allocation_pools dhcp_enabled dns_nameservers
                host_routes
        };
        save_option_fields! {
            self -> update: description gateway_ip name
        };
        let inner = api::update_subnet(&self.session, self.id(), update)?;
        self.dirty.clear();
        self.inner = inner;
        Ok(())
    }
}

impl Refresh for Subnet {
    /// Refresh the subnet.
    fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_subnet_by_id(&self.session, &self.inner.id)?;
        self.dirty.clear();
        Ok(())
    }
}

impl SubnetQuery {
    pub(crate) fn new(session: Arc<Session>) -> SubnetQuery {
        SubnetQuery {
            session,
            query: Query::new(),
            can_paginate: true,
            network: None,
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
    pub fn set_network<N: Into<NetworkRef>>(&mut self, value: N) {
        self.network = Some(value.into());
    }

    /// Filter by network.
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
    pub fn into_iter(self) -> ResourceIterator<SubnetQuery> {
        debug!("Fetching subnets with {:?}", self.query);
        ResourceIterator::new(self)
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

impl ResourceQuery for SubnetQuery {
    type Item = Subnet;

    const DEFAULT_LIMIT: usize = 50;

    fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_subnets(&self.session, &query)?
            .into_iter()
            .map(|item| Subnet::new(self.session.clone(), item))
            .collect())
    }

    fn validate(&mut self) -> Result<()> {
        if let Some(network) = self.network.take() {
            let verified = network.into_verified(&self.session)?;
            self.query.push_str("network_id", verified);
        }
        Ok(())
    }
}

impl NewSubnet {
    /// Start creating a subnet.
    pub(crate) fn new(session: Arc<Session>, network: NetworkRef, cidr: ipnet::IpNet) -> NewSubnet {
        NewSubnet {
            session,
            inner: protocol::Subnet::empty(cidr),
            network,
        }
    }

    /// Request creation of the subnet.
    pub fn create(mut self) -> Result<Subnet> {
        self.inner.network_id = self.network.into_verified(&self.session)?.into();
        self.inner.ip_version = match self.inner.cidr {
            ipnet::IpNet::V4(..) => protocol::IpVersion::V4,
            ipnet::IpNet::V6(..) => protocol::IpVersion::V6,
        };

        let subnet = api::create_subnet(&self.session, self.inner)?;
        Ok(Subnet::new(self.session, subnet))
    }

    creation_inner_vec! {
        #[doc = "Allocation pool(s) for the subnet (the default is the whole CIDR)."]
        add_allocation_pool, with_allocation_pool -> allocation_pools: protocol::AllocationPool
    }

    creation_inner_field! {
        #[doc = "Set CIDR of the subnet."]
        set_cidr, with_cidr -> cidr: ipnet::IpNet
    }

    creation_inner_field! {
        #[doc = "Set description of the subnet."]
        set_description, with_description -> description: optional String
    }

    creation_inner_field! {
        #[doc = "Configure whether DHCP is enabled (true by default)."]
        set_dhcp_enabled, with_dhcp_enabled -> dhcp_enabled: bool
    }

    creation_inner_vec! {
        #[doc = "DNS nameserver(s) for the subnet."]
        add_dns_nameserver, with_dns_nameserver -> dns_nameservers
    }

    creation_inner_vec! {
        #[doc = "Host route(s) for the subnet."]
        add_host_route, with_host_route -> host_routes: protocol::HostRoute
    }

    creation_inner_field! {
        #[doc = "Set IPv6 address assignment mode."]
        set_ipv6_address_mode, with_ipv6_address_mode
            -> ipv6_address_mode: optional protocol::Ipv6Mode
    }

    creation_inner_field! {
        #[doc = "Set IPv6 router advertisement mode."]
        set_ipv6_router_advertisement_mode, with_ipv6_router_advertisement_mode
            -> ipv6_router_advertisement_mode: optional protocol::Ipv6Mode
    }

    creation_inner_field! {
        #[doc = "Set a name for the subnet."]
        set_name, with_name -> name: optional String
    }

    /// Set the network of the subnet.
    pub fn set_network<N>(&mut self, value: N)
    where
        N: Into<NetworkRef>,
    {
        self.network = value.into();
    }

    /// Set the network of the subnet.
    pub fn with_network<N>(mut self, value: N) -> Self
    where
        N: Into<NetworkRef>,
    {
        self.set_network(value);
        self
    }
}

impl IntoFallibleIterator for SubnetQuery {
    type Item = Subnet;

    type Error = Error;

    type IntoIter = ResourceIterator<SubnetQuery>;

    fn into_fallible_iterator(self) -> Self::IntoIter {
        self.into_iter()
    }
}

impl From<Subnet> for SubnetRef {
    fn from(value: Subnet) -> SubnetRef {
        SubnetRef::new_verified(value.inner.id)
    }
}

#[cfg(feature = "network")]
impl IntoVerified for SubnetRef {
    /// Verify this reference and convert to an ID, if possible.
    fn into_verified(self, session: &Session) -> Result<SubnetRef> {
        Ok(if self.verified {
            self
        } else {
            SubnetRef::new_verified(api::get_subnet(session, &self.value)?.id)
        })
    }
}
