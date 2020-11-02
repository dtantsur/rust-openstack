// Copyright 2020 Martin Chlumsky <martin.chlumsky@gmail.com>
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

use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{FallibleIterator, IntoFallibleIterator};

use super::super::common::{
    DeletionWaiter, IntoVerified, Refresh, ResourceIterator, ResourceQuery, RouterRef,
};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, ErrorKind, Result, Sort};
use super::{api, protocol, Network};

/// A query to router list.
#[derive(Clone, Debug)]
pub struct RouterQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a single router.
#[derive(Clone, Debug)]
pub struct Router {
    session: Rc<Session>,
    inner: protocol::Router,
    dirty: HashSet<&'static str>,
}

/// A request to create a router
#[derive(Clone, Debug)]
pub struct NewRouter {
    session: Rc<Session>,
    inner: protocol::Router,
}

impl Router {
    /// Create a router object.
    fn new(session: Rc<Session>, inner: protocol::Router) -> Router {
        Router {
            session,
            inner,
            dirty: HashSet::new(),
        }
    }

    /// Load a Router object.
    pub(crate) fn load<Id: AsRef<str>>(session: Rc<Session>, id: Id) -> Result<Router> {
        let inner = api::get_router(&session, id)?;
        Ok(Router::new(session, inner))
    }

    transparent_property! {
        #[doc = "The administrative state of the router."]
        admin_state_up: bool
    }

    update_field! {
        #[doc = "Set the administrative state of the router."]
        set_admin_state_up, with_admin_state_up -> admin_state_up: bool
    }

    transparent_property! {
        #[doc = "Availability zone candidates for the router"]
        availability_zone_hints: ref Vec<String>
    }

    transparent_property! {
        #[doc = "The availability zones for the router (if available)."]
        availability_zones: ref Vec<String>
    }

    transparent_property! {
        #[doc = "The associated conntrack helper resources for the router."]
        conntrack_helpers: ref Vec<protocol::ConntrackHelper>
    }

    transparent_property! {
        #[doc = "Creation data and time (if available)."]
        created_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        #[doc = "Router description."]
        description: ref Option<String>
    }

    update_field! {
        #[doc = "Update the description."]
        set_description, with_description -> description: optional String
    }

    transparent_property! {
        #[doc = "Indicates if the router is distributed."]
        distributed: Option<bool>
    }

    update_field! {
        #[doc = "Update whether this is a distributed router."]
        set_distributed, with_distributed -> distributed: optional bool
    }

    transparent_property! {
        #[doc = "External gateway information."]
        external_gateway: ref Option<protocol::ExternalGateway>
    }

    /// Get external network associated with this router.
    ///
    /// Fails if external gateway information is not provided.
    pub fn external_network(&self) -> Result<Network> {
        if let Some(ref gw) = self.inner.external_gateway {
            Network::load(self.session.clone(), &gw.network_id)
        } else {
            Err(Error::new(
                ErrorKind::ResourceNotFound,
                format!("No external gateway for router {}", self.inner.id),
            ))
        }
    }

    update_field! {
        #[doc = "Update the external gateway information."]
        set_external_gateway, with_external_gateway -> external_gateway: optional protocol::ExternalGateway
    }

    transparent_property! {
        #[doc = "Flavor associated with router."]
        flavor_id:  ref Option<String>
    }

    transparent_property! {
        #[doc = "Indicates if the router is highly-available."]
        ha: Option<bool>
    }

    update_field! {
        #[doc = "Update whether this is a highly-available router."]
        set_ha, with_ha -> ha: optional bool
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Router name."]
        name: ref Option<String>
    }

    update_field! {
        #[doc = "Update the name."]
        set_name, with_name -> name: optional String
    }

    transparent_property! {
        #[doc = "Project ID."]
        project_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "Revision number."]
        revision_number: Option<u32>
    }

    transparent_property! {
        #[doc = "Extra routes."]
        routes: ref Option<Vec<protocol::HostRoute>>
    }

    update_field! {
        #[doc = "Update extra routes."]
        set_routes, with_routes -> routes: optional Vec<protocol::HostRoute>
    }

    transparent_property! {
        #[doc = "ID of the service type associated to the router."]
        service_type_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "Status of the router."]
        status: protocol::RouterStatus
    }

    transparent_property! {
        #[doc = "Tags."]
        tags: ref Option<Vec<String>>
    }

    transparent_property! {
        #[doc = "Last update data and time (if available)."]
        updated_at: Option<DateTime<FixedOffset>>
    }

    /// Delete the router.
    pub fn delete(self) -> Result<DeletionWaiter<Router>> {
        api::delete_router(&self.session, &self.inner.id)?;
        Ok(DeletionWaiter::new(
            self,
            Duration::new(60, 0),
            Duration::new(1, 0),
        ))
    }

    /// Whether the router is modified.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Save the changes to the router.
    pub fn save(&mut self) -> Result<()> {
        let mut update = protocol::RouterUpdate::default();
        if let Some(ref gw) = self.inner.external_gateway {
            update.external_gateway = Some(gw.clone().into_verified(&self.session)?);
        }
        save_fields! {
            self -> update: admin_state_up
        };
        save_option_fields! {
            self -> update: description distributed ha name routes
        };
        let inner = api::update_router(&self.session, self.id(), update)?;
        self.dirty.clear();
        self.inner = inner;
        Ok(())
    }

    /// Add an interface to the router.
    pub fn add_router_interface(
        &mut self,
        subnet_id: Option<&String>,
        port_id: Option<&String>,
    ) -> Result<()> {
        api::add_router_interface(&self.session, self.id(), subnet_id, port_id)
    }

    /// Remove an interface from the router.
    pub fn remove_router_interface(
        &mut self,
        subnet_id: Option<&String>,
        port_id: Option<&String>,
    ) -> Result<()> {
        api::remove_router_interface(&self.session, self.id(), subnet_id, port_id)
    }

    /// Add route to router.
    pub fn add_extra_routes(&mut self, routes: Vec<protocol::HostRoute>) -> Result<()> {
        api::add_extra_routes(&self.session, self.id(), routes)
    }

    /// Remove route from router.
    pub fn remove_extra_routes(&mut self, routes: Vec<protocol::HostRoute>) -> Result<()> {
        api::remove_extra_routes(&self.session, self.id(), routes)
    }
}

impl Refresh for Router {
    /// Refresh the router.
    fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_router_by_id(&self.session, &self.inner.id)?;
        self.dirty.clear();
        Ok(())
    }
}

impl RouterQuery {
    pub(crate) fn new(session: Rc<Session>) -> RouterQuery {
        RouterQuery {
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
    pub fn sort_by(mut self, sort: Sort<protocol::RouterSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.query.push_str("sort_key", field);
        self.query.push("sort_dir", direction);
        self
    }

    /// Filter by router name (a database regular expression).
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
    pub fn into_iter(self) -> ResourceIterator<RouterQuery> {
        debug!("Fetching routers with {:?}", self.query);
        ResourceIterator::new(self)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Router>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Router> {
        debug!("Fetching one router with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl ResourceQuery for RouterQuery {
    type Item = Router;

    const DEFAULT_LIMIT: usize = 50;

    fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_routers(&self.session, &query)?
            .into_iter()
            .map(|item| Router::new(self.session.clone(), item))
            .collect())
    }
}

impl NewRouter {
    /// Start creating a router.
    pub(crate) fn new(session: Rc<Session>) -> NewRouter {
        NewRouter {
            session,
            inner: protocol::Router::default(),
        }
    }

    /// Request creation of a router.
    pub fn create(self) -> Result<Router> {
        let inner = api::create_router(&self.session, self.inner.into_verified(&self.session)?)?;
        Ok(Router::new(self.session, inner))
    }

    creation_inner_field! {
        #[doc = "Set administrative status for the router."]
        set_admin_state_up, with_admin_state_up -> admin_state_up: bool
    }

    creation_inner_field! {
        #[doc = "Set the availability zone candidates for the router."]
        set_availability_zone_hints, with_availability_zone_hints -> availability_zone_hints: Vec<String>
    }

    creation_inner_field! {
        #[doc = "Set description of the router."]
        set_description, with_description -> description: optional String
    }

    creation_inner_field! {
        #[doc = "Set whether the router is distributed."]
        set_distributed, with_distributed -> distributed: optional bool
    }

    creation_inner_field! {
        #[doc = "Set the external gateway information."]
        set_external_gateway, with_external_gateway -> external_gateway: optional protocol::ExternalGateway
    }

    creation_inner_field! {
        #[doc = "Set the ID of the flavor associated with the router."]
        set_flavor_id, with_flavor_id -> flavor_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set whether the router is highly-available."]
        set_ha, with_ha -> ha: optional bool
    }

    creation_inner_field! {
        #[doc = "Set a name for the router."]
        set_name, with_name -> name: optional String
    }

    creation_inner_field! {
        #[doc = "Set a project id for the router."]
        set_project_id, with_project_id -> project_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set the ID of the service type associated with the router."]
        set_service_type_id, with_service_type_id -> service_type_id: optional String
    }
}

impl IntoFallibleIterator for RouterQuery {
    type Item = Router;

    type Error = Error;

    type IntoFallibleIter = ResourceIterator<RouterQuery>;

    fn into_fallible_iter(self) -> Self::IntoFallibleIter {
        self.into_iter()
    }
}

impl From<Router> for RouterRef {
    fn from(value: Router) -> RouterRef {
        RouterRef::new_verified(value.inner.id)
    }
}

#[cfg(feature = "network")]
impl IntoVerified for RouterRef {
    /// Verify this reference and convert to an ID, if possible.
    fn into_verified(self, session: &Session) -> Result<RouterRef> {
        Ok(if self.verified {
            self
        } else {
            RouterRef::new_verified(api::get_router(session, &self.value)?.id)
        })
    }
}
