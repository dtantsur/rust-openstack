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

//! Network management via Network API.

use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{FallibleIterator, IntoFallibleIterator};

use super::super::common::{
    DeletionWaiter, IntoVerified, NetworkRef, Refresh, ResourceIterator, ResourceQuery,
};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, Result, Sort};
use super::{api, protocol};

/// A query to network list.
#[derive(Clone, Debug)]
pub struct NetworkQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a single network.
#[derive(Clone, Debug)]
pub struct Network {
    session: Rc<Session>,
    inner: protocol::Network,
    dirty: HashSet<&'static str>,
}

/// A request to create a network
#[derive(Clone, Debug)]
pub struct NewNetwork {
    session: Rc<Session>,
    inner: protocol::Network,
}

impl Network {
    /// Create a network object.
    fn new(session: Rc<Session>, inner: protocol::Network) -> Network {
        Network {
            session,
            inner,
            dirty: HashSet::new(),
        }
    }

    /// Load a Network object.
    pub(crate) fn load<Id: AsRef<str>>(session: Rc<Session>, id: Id) -> Result<Network> {
        let inner = api::get_network(&session, id)?;
        Ok(Network::new(session, inner))
    }

    transparent_property! {
        #[doc = "The administrative state of the network."]
        admin_state_up: bool
    }

    update_field! {
        #[doc = "Set the administrative state of the network."]
        set_admin_state_up, with_admin_state_up -> admin_state_up: bool
    }

    transparent_property! {
        #[doc = "The availability zones for the network (if available)."]
        availability_zones: ref Vec<String>
    }

    transparent_property! {
        #[doc = "Creation data and time (if available)."]
        created_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        #[doc = "Network description."]
        description: ref Option<String>
    }

    update_field! {
        #[doc = "Update the description."]
        set_description, with_description -> description: optional String
    }

    transparent_property! {
        #[doc = "DNS domain for the network (if available)."]
        dns_domain: ref Option<String>
    }

    update_field! {
        #[doc = "Update the DNS domain."]
        set_dns_domain, with_dns_domain -> dns_domain: optional String
    }

    transparent_property! {
        #[doc = "Whether the network is external (if available)."]
        external: Option<bool>
    }

    update_field! {
        #[doc = "Configure whether the network is external."]
        set_external, with_external -> external: optional bool
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Whether the network is the default pool (if available)."]
        is_default: Option<bool>
    }

    update_field! {
        #[doc = "Configure whether the network is the default pool."]
        set_default, with_default -> is_default: optional bool
    }

    transparent_property! {
        #[doc = "Whether there is L2 connectivity throughout the Network."]
        l2_adjacency: Option<bool>
    }

    transparent_property! {
        #[doc = "Network MTU (if available)."]
        mtu: Option<u32>
    }

    update_field! {
        #[doc = "Set the network MTU."]
        set_mtu, with_mtu -> mtu: optional u32
    }

    transparent_property! {
        #[doc = "Network name."]
        name: ref Option<String>
    }

    update_field! {
        #[doc = "Update the name."]
        set_name, with_name -> name: optional String
    }

    transparent_property! {
        #[doc = "Whether port security is enabled by default."]
        port_security_enabled: Option<bool>
    }

    update_field! {
        #[doc = "Configure whether port security is enabled by default."]
        set_port_security_enabled, with_port_security_enabled
            -> port_security_enabled: optional bool
    }

    transparent_property! {
        #[doc = "Whether the network is shared."]
        shared: bool
    }

    update_field! {
        #[doc = "Configure whether the network is shared."]
        set_shared, with_shared -> shared: bool
    }

    transparent_property! {
        #[doc = "Status of the network."]
        status: protocol::NetworkStatus
    }

    transparent_property! {
        #[doc = "Last update data and time (if available)."]
        updated_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        #[doc = "VLAN transparency mode of the network."]
        vlan_transparent: Option<bool>
    }

    /// Delete the network.
    pub fn delete(self) -> Result<DeletionWaiter<Network>> {
        api::delete_network(&self.session, &self.inner.id)?;
        Ok(DeletionWaiter::new(
            self,
            Duration::new(60, 0),
            Duration::new(1, 0),
        ))
    }

    /// Whether the network is modified.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Save the changes to the network.
    pub fn save(&mut self) -> Result<()> {
        let mut update = protocol::NetworkUpdate::default();
        save_fields! {
            self -> update: admin_state_up shared
        };
        save_option_fields! {
            self -> update: description external dns_domain is_default mtu name
                port_security_enabled
        };
        let inner = api::update_network(&self.session, self.id(), update)?;
        self.dirty.clear();
        self.inner = inner;
        Ok(())
    }
}

impl Refresh for Network {
    /// Refresh the network.
    fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_network_by_id(&self.session, &self.inner.id)?;
        self.dirty.clear();
        Ok(())
    }
}

impl NetworkQuery {
    pub(crate) fn new(session: Rc<Session>) -> NetworkQuery {
        NetworkQuery {
            session,
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
    pub fn sort_by(mut self, sort: Sort<protocol::NetworkSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.query.push_str("sort_key", field);
        self.query.push("sort_dir", direction);
        self
    }

    /// Filter by network name (a database regular expression).
    pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
        self.query.push_str("name", value);
        self
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<NetworkQuery> {
        debug!("Fetching networks with {:?}", self.query);
        ResourceIterator::new(self)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Network>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Network> {
        debug!("Fetching one network with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl ResourceQuery for NetworkQuery {
    type Item = Network;

    const DEFAULT_LIMIT: usize = 50;

    fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_networks(&self.session, &query)?
            .into_iter()
            .map(|item| Network::new(self.session.clone(), item))
            .collect())
    }
}

impl NewNetwork {
    /// Start creating a network.
    pub(crate) fn new(session: Rc<Session>) -> NewNetwork {
        NewNetwork {
            session,
            inner: protocol::Network::default(),
        }
    }

    /// Request creation of a network.
    pub fn create(self) -> Result<Network> {
        let inner = api::create_network(&self.session, self.inner)?;
        Ok(Network::new(self.session, inner))
    }

    creation_inner_field! {
        #[doc = "Set administrative status for the network."]
        set_admin_state_up, with_admin_state_up -> admin_state_up: bool
    }

    creation_inner_field! {
        #[doc = "Configure whether this network is default."]
        set_default, with_default -> is_default: optional bool
    }

    creation_inner_field! {
        #[doc = "Set description of the network."]
        set_description, with_description -> description: optional String
    }

    creation_inner_field! {
        #[doc = "Set DNS domain for the network."]
        set_dns_domain, with_dns_domain -> dns_domain: optional String
    }

    creation_inner_field! {
        #[doc = "Configure whether this network is external."]
        set_external, with_external -> external: optional bool
    }

    creation_inner_field! {
        #[doc = "Set MTU for the network."]
        set_mtu, with_mtu -> mtu: optional u32
    }

    creation_inner_field! {
        #[doc = "Set a name for the network."]
        set_name, with_name -> name: optional String
    }

    creation_inner_field! {
        #[doc = "Configure whether port security is enabled by default."]
        set_port_security_enabled, with_port_security_enabled
            -> port_security_enabled: optional bool
    }

    creation_inner_field! {
        #[doc = "Configure VLAN transparency mode of the network."]
        set_vlan_transparent, with_vlan_transparent
            -> vlan_transparent: optional bool
    }
}

impl IntoFallibleIterator for NetworkQuery {
    type Item = Network;

    type Error = Error;

    type IntoFallibleIter = ResourceIterator<NetworkQuery>;

    fn into_fallible_iter(self) -> Self::IntoFallibleIter {
        self.into_iter()
    }
}

impl From<Network> for NetworkRef {
    fn from(value: Network) -> NetworkRef {
        NetworkRef::new_verified(value.inner.id)
    }
}

#[cfg(feature = "network")]
impl IntoVerified for NetworkRef {
    /// Verify this reference and convert to an ID, if possible.
    fn into_verified(self, session: &Session) -> Result<NetworkRef> {
        Ok(if self.verified {
            self
        } else {
            NetworkRef::new_verified(api::get_network(session, &self.value)?.id)
        })
    }
}
