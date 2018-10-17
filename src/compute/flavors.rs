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

//! Flavor management via Compute API.

use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, Result};
use super::super::common::{self, FlavorRef, ListResources, Refresh, ResourceId,
                           ResourceIterator};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::protocol;


/// Structure representing a flavor.
#[derive(Clone, Debug)]
pub struct Flavor {
    session: Rc<Session>,
    inner: protocol::Flavor,
    extra_specs: HashMap<String, String>,
}

/// Structure representing a summary of a flavor.
#[derive(Clone, Debug)]
pub struct FlavorSummary {
    session: Rc<Session>,
    inner: common::protocol::IdAndName,
}

/// A query to server list.
#[derive(Clone, Debug)]
pub struct FlavorQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}


impl Flavor {
    /// Create a flavor object.
    pub(crate) fn new(session: Rc<Session>, mut inner: protocol::Flavor)
            -> Result<Flavor> {
        let extra_specs = match inner.extra_specs.take() {
            Some(es) => es,
            None => session.get_extra_specs_by_flavor_id(&inner.id)?
        };

        Ok(Flavor {
            session: session,
            inner: inner,
            extra_specs: extra_specs,
        })
    }

    /// Load a Flavor object.
    pub(crate) fn load<Id: AsRef<str>>(session: Rc<Session>, id: Id)
            -> Result<Flavor> {
        let inner = session.get_flavor(id)?;
        Flavor::new(session, inner)
    }

    /// Get ephemeral disk size in GiB.
    ///
    /// Returns `0` when ephemeral disk was not requested.
    pub fn emphemeral_size(&self) -> u64 {
        self.inner.ephemeral
    }

    /// Extra specs of the flavor.
    pub fn extra_specs(&self) -> &HashMap<String, String> {
        &self.extra_specs
    }

    /// Get a reference to flavor unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Whether the flavor is public.
    pub fn is_public(&self) -> bool {
        self.inner.is_public
    }

    /// Get a reference to flavor name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get RAM size in MiB.
    pub fn ram_size(&self) -> u64 {
        self.inner.ram
    }

    /// Get root disk size in GiB.
    pub fn root_size(&self) -> u64 {
        self.inner.disk
    }

    /// Get swap size in MiB.
    ///
    /// Returns `0` when swap was not requested.
    pub fn swap_size(&self) -> u64 {
        self.inner.swap
    }

    /// Get VCPU count.
    pub fn vcpu_count(&self) -> u32 {
        self.inner.vcpus
    }
}

impl Refresh for Flavor {
    /// Refresh the flavor.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_flavor(&self.inner.id)?;
        Ok(())
    }
}

impl FlavorSummary {
    /// Get a reference to flavor unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to flavor name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(&self) -> Result<Flavor> {
        Flavor::load(self.session.clone(), &self.inner.id)
    }
}

impl FlavorQuery {
    pub(crate) fn new(session: Rc<Session>) -> FlavorQuery {
        FlavorQuery {
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

    /// Convert this query into an iterator executing the request.
    ///
    /// This iterator yields only `FlavorSummary` objects, containing
    /// IDs and names. Use `into_iter_detailed` for full `Flavor` objects.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<FlavorSummary> {
        debug!("Fetching flavors with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// This iterator yields full `Flavor` objects. If you only need IDs
    /// and/or names, use `into_iter` to save bandwidth.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter_detailed(self) -> ResourceIterator<Flavor> {
        debug!("Fetching flavor details with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<FlavorSummary>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<FlavorSummary> {
        debug!("Fetching one flavor with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}


impl ResourceId for FlavorSummary {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for FlavorSummary {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<FlavorSummary>> {
        Ok(session.list_flavors(&query)?.into_iter().map(|item| FlavorSummary {
            session: session.clone(),
            inner: item
        }).collect())
    }
}

impl ResourceId for Flavor {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl ListResources for Flavor {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<Flavor>> {
        let flavors = session.list_flavors_detail(&query)?;
        let mut result = Vec::with_capacity(flavors.len());
        for item in flavors {
            result.push(Flavor::new(session.clone(), item)?);
        }
        Ok(result)
    }
}

impl IntoFallibleIterator for FlavorQuery {
    type Item = FlavorSummary;

    type Error = Error;

    type IntoIter = ResourceIterator<FlavorSummary>;

    fn into_fallible_iterator(self) -> ResourceIterator<FlavorSummary> {
        self.into_iter()
    }
}

impl From<Flavor> for FlavorRef {
    fn from(value: Flavor) -> FlavorRef {
        FlavorRef::new_verified(value.inner.id)
    }
}

impl From<FlavorSummary> for FlavorRef {
    fn from(value: FlavorSummary) -> FlavorRef {
        FlavorRef::new_verified(value.inner.id)
    }
}

impl FlavorRef {
    /// Verify this reference and convert to an ID, if possible.
    #[cfg(feature = "compute")]
    pub(crate) fn into_verified(self, session: &Session) -> Result<String> {
        Ok(if self.verified {
            self.value
        } else {
            session.get_flavor(&self.value)?.id
        })
    }
}

impl From<Flavor> for protocol::ServerFlavor {
    fn from(value: Flavor) -> protocol::ServerFlavor {
        protocol::ServerFlavor {
            ephemeral_size: value.inner.ephemeral,
            extra_specs: Some(value.extra_specs),
            original_name: value.inner.name,
            ram_size: value.inner.ram,
            root_size: value.inner.disk,
            swap_size: value.inner.swap,
            vcpu_count: value.inner.vcpus,
        }
    }
}
