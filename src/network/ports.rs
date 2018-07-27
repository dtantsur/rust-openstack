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

//! Ports management via Port API.

use std::rc::Rc;
use std::fmt::Debug;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use eui48::MacAddress;
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, Result, Sort};
use super::super::common::{DeletionWaiter, ListResources, NetworkRef, PortRef,
                           Refresh, ResourceId, ResourceIterator};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::{protocol, Network};


/// A query to port list.
#[derive(Clone, Debug)]
pub struct PortQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a port - a virtual NIC.
#[derive(Clone, Debug)]
pub struct Port {
    session: Rc<Session>,
    inner: protocol::Port
}

/// A request to create a port
#[derive(Clone, Debug)]
pub struct NewPort {
    session: Rc<Session>,
    inner: protocol::Port,
    network: NetworkRef,
}

impl Port {
    /// Load a Port object.
    pub(crate) fn new<Id: AsRef<str>>(session: Rc<Session>, id: Id)
            -> Result<Port> {
        let inner = session.get_port(id)?;
        Ok(Port {
            session: session,
            inner: inner
        })
    }

    transparent_property! {
        #[doc = "The administrative state of the port."]
        admin_state_up: bool
    }

    /// Whether the `device_owner` is a Compute server.
    pub fn attached_to_server(&self) -> bool {
        match self.inner.device_owner {
            Some(ref x) => x.starts_with("compute:"),
            None => false
        }
    }

    transparent_property! {
        #[doc = "Creation data and time (if available)."]
        created_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        #[doc = "Port description."]
        description: ref Option<String>
    }

    transparent_property! {
        #[doc = "ID of object (server, router, etc) to which this port is attached."]
        device_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "Type of object to which this port is attached."]
        device_owner: ref Option<String>
    }

    transparent_property! {
        #[doc = "DNS domain for the port (if available)."]
        dns_domain: ref Option<String>
    }

    transparent_property! {
        #[doc = "DNS domain for the port (if available)."]
        dns_name: ref Option<String>
    }

    transparent_property! {
        #[doc = "DHCP options configured for this port."]
        extra_dhcp_opts: ref Vec<protocol::PortExtraDhcpOption>
    }

    transparent_property! {
        #[doc = "Fixed IP addresses of the port."]
        fixed_ips: ref Vec<protocol::PortIpAddress>
    }

    transparent_property! {
        #[doc = "MAC address of the port."]
        mac_address: MacAddress
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Port name."]
        name: ref Option<String>
    }

    /// Get network associated with this port.
    pub fn network(&self) -> Result<Network> {
        Network::new(self.session.clone(), &self.inner.network_id)
    }

    transparent_property! {
        #[doc = "ID of the network this port belongs to."]
        network_id: ref String
    }

    transparent_property! {
        #[doc = "Port status."]
        status: protocol::NetworkStatus
    }

    transparent_property! {
        #[doc = "Last update data and time (if available)."]
        updated_at: Option<DateTime<FixedOffset>>
    }

    /// Delete the port.
    pub fn delete(self) -> Result<DeletionWaiter<Port>> {
        self.session.delete_port(&self.inner.id)?;
        Ok(DeletionWaiter::new(self, Duration::new(60, 0), Duration::new(1, 0)))
    }
}

impl Refresh for Port {
    /// Refresh the port.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_port(&self.inner.id)?;
        Ok(())
    }
}

impl PortQuery {
    pub(crate) fn new(session: Rc<Session>) -> PortQuery {
        PortQuery {
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
    pub fn sort_by(mut self, sort: Sort<protocol::PortSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.query.push_str("sort_key", field);
        self.query.push("sort_dir", direction);
        self
    }

    query_filter! {
        #[doc = "Filter by administrative state."]
        set_admin_state_up, with_admin_state_up -> admin_state_up: bool
    }

    query_filter! {
        #[doc = "Filter by description."]
        set_description, with_description -> description
    }

    query_filter! {
        #[doc = "Filter by the ID of the object attached to the port."]
        set_device_id, with_device_id -> device_id
    }

    query_filter! {
        #[doc = "Filter by the ID of the object attached to the port."]
        set_device_owner, with_device_owner -> device_owner
    }

    query_filter! {
        #[doc = "Filter by MAC address."]
        set_mac_address, with_mac_address -> mac_address
    }

    query_filter! {
        #[doc = "Filter by port name."]
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

    query_filter! {
        #[doc = "Filter by status."]
        set_status, with_status -> status: protocol::NetworkStatus
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<Port> {
        debug!("Fetching ports with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Port>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Port> {
        debug!("Fetching one port with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl NewPort {
    /// Start creating a port.
    pub(crate) fn new(session: Rc<Session>, network: NetworkRef)
            -> NewPort {
        NewPort {
            session: session,
            inner: protocol::Port {
                admin_state_up: true,
                created_at: None,
                description: None,
                device_id: None,
                device_owner: None,
                dns_domain: None,
                dns_name: None,
                extra_dhcp_opts: Vec::new(),
                fixed_ips: Vec::new(),
                id: String::new(),
                mac_address: Default::default(),
                name: None,
                // Will be replaced in create()
                network_id: String::new(),
                project_id: None,
                security_groups: Vec::new(),
                // Dummy value, not used when serializing
                status: protocol::NetworkStatus::Active,
                updated_at: None,
            },
            network: network,
        }
    }

    /// Request creation of the port.
    pub fn create(mut self) -> Result<Port> {
        self.inner.network_id = self.network.into_verified(&self.session)?;
        let port = self.session.create_port(self.inner)?;
        Ok(Port {
            session: self.session,
            inner: port
        })
    }

    creation_inner_field! {
        #[doc = "Set administrative status for the port."]
        set_admin_state_up, with_admin_state_up -> admin_state_up: bool
    }

    creation_inner_field! {
        #[doc = "Set description of the port."]
        set_description, with_description -> description: optional String
    }

    creation_inner_field! {
        #[doc = "Set device ID of the port."]
        set_device_id, with_device_id -> device_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set device owner of the port."]
        set_device_owner, with_device_owner -> device_owner: optional String
    }

    creation_inner_field! {
        #[doc = "Set DNS domain for the port."]
        set_dns_domain, with_dns_domain -> dns_domain: optional String
    }

    creation_inner_field! {
        #[doc = "Set DNS name for the port."]
        set_dns_name, with_dns_name -> dns_name: optional String
    }

    /// Extra DHCP options to configure on the port.
    pub fn extra_dhcp_opts(&mut self) -> &mut Vec<protocol::PortExtraDhcpOption> {
        &mut self.inner.extra_dhcp_opts
    }

    creation_inner_field! {
        #[doc = "Set extra DHCP options to configure on the port."]
        set_extra_dhcp_opts, with_extra_dhcp_opts -> extra_dhcp_opts:
            Vec<protocol::PortExtraDhcpOption>
    }

    // TODO(dtantsur): fixed IPs

    creation_inner_field! {
        #[doc = "Set MAC address for the port (generated otherwise)."]
        set_mac_address, with_mac_address -> mac_address: MacAddress
    }

    creation_inner_field! {
        #[doc = "Set a name for the port."]
        set_name, with_name -> name: optional String
    }

    // TODO(dtantsur): security groups
}

impl ResourceId for Port {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for Port {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<Port>> {
        Ok(session.list_ports(&query)?.into_iter().map(|item| Port {
            session: session.clone(),
            inner: item
        }).collect())
    }
}

impl IntoFallibleIterator for PortQuery {
    type Item = Port;

    type Error = Error;

    type IntoIter = ResourceIterator<Port>;

    fn into_fallible_iterator(self) -> ResourceIterator<Port> {
        self.into_iter()
    }
}

impl From<Port> for PortRef {
    fn from(value: Port) -> PortRef {
        PortRef::new_verified(value.inner.id)
    }
}

impl PortRef {
    /// Verify this reference and convert to an ID, if possible.
    #[cfg(feature = "network")]
    pub(crate) fn into_verified(self, session: &Session) -> Result<String> {
        Ok(if self.verified {
            self.value
        } else {
            session.get_port(&self.value)?.id
        })
    }
}
