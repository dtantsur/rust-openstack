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

use std::fmt::Debug;

use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, Result};
use super::super::service::{ListResources, ResourceId, ResourceIterator};
use super::super::session::Session;
use super::super::types::FlavorId;
use super::super::utils::{self, Query};
use super::base::V2API;
use super::protocol;


/// Structure representing a flavor.
#[derive(Clone, Debug)]
pub struct Flavor<'session> {
    session: &'session Session,
    inner: protocol::Flavor
}

/// Structure representing a summary of a flavor.
#[derive(Clone, Debug)]
pub struct FlavorSummary<'session> {
    session: &'session Session,
    inner: protocol::FlavorSummary
}

/// A query to server list.
#[derive(Clone, Debug)]
pub struct FlavorQuery<'session> {
    session: &'session Session,
    query: Query,
    can_paginate: bool,
}


impl<'session> Flavor<'session> {
    /// Load a Flavor object.
    pub(crate) fn new<Id: AsRef<str>>(session: &'session Session, id: Id)
            -> Result<Flavor<'session>> {
        let inner = session.get_flavor(id)?;
        Ok(Flavor {
            session: session,
            inner: inner
        })
    }

    /// Refresh the server.
    pub fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_flavor(&self.inner.id)?;
        Ok(())
    }

    /// Get ephemeral disk size in GiB.
    ///
    /// Returns `0` when ephemeral disk was not requested.
    pub fn emphemeral_size(&self) -> u64 {
        self.inner.ephemeral
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

impl<'session> FlavorSummary<'session> {
    /// Get a reference to flavor unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to flavor name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(&self) -> Result<Flavor<'session>> {
        Flavor::new(self.session, &self.inner.id)
    }
}

impl<'session> FlavorQuery<'session> {
    pub(crate) fn new(session: &'session Session) -> FlavorQuery<'session> {
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
    pub fn into_iter(self) -> ResourceIterator<'session, FlavorSummary<'session>> {
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
    pub fn into_iter_detailed(self) -> ResourceIterator<'session, Flavor<'session>> {
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<FlavorSummary<'session>>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<FlavorSummary<'session>> {
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        utils::fetch_one(self)
    }
}


impl<'session> ResourceId for FlavorSummary<'session> {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl<'session> ListResources<'session> for FlavorSummary<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<FlavorSummary<'session>>> {
        Ok(session.list_flavors(&query)?.into_iter().map(|item| FlavorSummary {
            session: session,
            inner: item
        }).collect())
    }
}

impl<'session> ResourceId for Flavor<'session> {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl<'session> ListResources<'session> for Flavor<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<Flavor<'session>>> {
        Ok(session.list_flavors_detail(&query)?.into_iter().map(|item| Flavor {
            session: session,
            inner: item
        }).collect())
    }
}

impl<'session> IntoFallibleIterator for FlavorQuery<'session> {
    type Item = FlavorSummary<'session>;

    type Error = Error;

    type IntoIter = ResourceIterator<'session, FlavorSummary<'session>>;

    fn into_fallible_iterator(self) -> ResourceIterator<'session, FlavorSummary<'session>> {
        self.into_iter()
    }
}

impl<'session> From<Flavor<'session>> for FlavorId {
    fn from(value: Flavor<'session>) -> FlavorId {
        FlavorId::from(value.inner.id)
    }
}

impl<'session> From<FlavorSummary<'session>> for FlavorId {
    fn from(value: FlavorSummary<'session>) -> FlavorId {
        FlavorId::from(value.inner.id)
    }
}
