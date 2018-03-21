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

//! Key pair management via Compute API.

use std::fmt::Debug;

use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, Result};
use super::super::common::{ListResources, Refresh, ResourceId,
                           ResourceIterator};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::protocol;


/// Structure representing a key pair.
#[derive(Clone, Debug)]
pub struct KeyPair<'session> {
    session: &'session Session,
    inner: protocol::KeyPair
}

/// A query to server list.
#[derive(Clone, Debug)]
pub struct KeyPairQuery<'session> {
    session: &'session Session,
    query: Query,
    can_paginate: bool,
}


impl<'session> KeyPair<'session> {
    /// Load a KeyPair object.
    pub(crate) fn new<Id: AsRef<str>>(session: &'session Session, id: Id)
            -> Result<KeyPair<'session>> {
        let inner = session.get_keypair(id)?;
        Ok(KeyPair {
            session: session,
            inner: inner
        })
    }

    transparent_property! {
        #[doc = "Key pair fingerprint."]
        fingerprint: ref String
    }

    transparent_property! {
        #[doc = "Key pair type, if available."]
        key_type: ref Option<protocol::KeyPairType>
    }

    transparent_property! {
        #[doc = "Key pair name."]
        name: ref String
    }
}

impl<'session> Refresh for KeyPair<'session> {
    /// Refresh the keypair.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_keypair(&self.inner.name)?;
        Ok(())
    }
}

impl<'session> KeyPairQuery<'session> {
    pub(crate) fn new(session: &'session Session) -> KeyPairQuery<'session> {
        KeyPairQuery {
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
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<'session, KeyPair<'session>> {
        debug!("Fetching key pairs with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<KeyPair<'session>>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<KeyPair<'session>> {
        debug!("Fetching one key pair with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl<'session> ResourceId for KeyPair<'session> {
    fn resource_id(&self) -> String {
        self.name().clone()
    }
}

impl<'session> ListResources<'session> for KeyPair<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<KeyPair<'session>>> {
        Ok(session.list_keypairs(&query)?.into_iter().map(|item| KeyPair {
            session: session,
            inner: item
        }).collect())
    }
}

impl<'session> IntoFallibleIterator for KeyPairQuery<'session> {
    type Item = KeyPair<'session>;

    type Error = Error;

    type IntoIter = ResourceIterator<'session, KeyPair<'session>>;

    fn into_fallible_iterator(self) -> ResourceIterator<'session, KeyPair<'session>> {
        self.into_iter()
    }
}
