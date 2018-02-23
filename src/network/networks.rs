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

use std::fmt::Debug;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, Result, Sort};
use super::super::service::{ListResources, ResourceId, ResourceIterator};
use super::super::session::Session;
use super::super::types;
use super::super::utils::{self, Query};
use super::base::V2API;
use super::protocol;


/// A query to network list.
#[derive(Clone, Debug)]
pub struct NetworkQuery<'session> {
    session: &'session Session,
    query: Query,
    can_paginate: bool,
}

/// Structure representing a single network.
#[derive(Clone, Debug)]
pub struct Network<'session> {
    session: &'session Session,
    inner: protocol::Network
}

impl<'session> Network<'session> {
    /// Load a Network object.
    pub(crate) fn new<Id: AsRef<str>>(session: &'session Session, id: Id)
            -> Result<Network<'session>> {
        let inner = session.get_network(id)?;
        Ok(Network {
            session: session,
            inner: inner
        })
    }

    /// Refresh the network.
    pub fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_network(&self.inner.id)?;
        Ok(())
    }

    /// Get a reference to creation date and time.
    pub fn created_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.created_at
    }

    /// Get a reference to network unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to network name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get a reference to last update date and time.
    pub fn updated_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.updated_at
    }
}

impl<'session> NetworkQuery<'session> {
    pub(crate) fn new(session: &'session Session) -> NetworkQuery<'session> {
        NetworkQuery {
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
    pub fn into_iter(self) -> ResourceIterator<'session, Network<'session>> {
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Network<'session>>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Network<'session>> {
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        utils::fetch_one(self)
    }
}

impl<'session> ResourceId for Network<'session> {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl<'session> ListResources<'session> for Network<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<Network<'session>>> {
        Ok(session.list_networks(&query)?.into_iter().map(|item| Network {
            session: session,
            inner: item
        }).collect())
    }
}

impl<'session> IntoFallibleIterator for NetworkQuery<'session> {
    type Item = Network<'session>;

    type Error = Error;

    type IntoIter = ResourceIterator<'session, Network<'session>>;

    fn into_fallible_iterator(self) -> ResourceIterator<'session, Network<'session>> {
        self.into_iter()
    }
}

impl<'session> From<Network<'session>> for types::NetworkRef {
    fn from(value: Network<'session>) -> types::NetworkRef {
        types::NetworkRef::from(value.inner.id)
    }
}