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
use std::io;
use std::rc::Rc;

use fallible_iterator::{IntoFallibleIterator, FallibleIterator};
use serde::Serialize;

use super::super::{Error, ErrorKind, Result};
use super::super::common::{KeyPairRef, ListResources, Refresh, ResourceId,
                           ResourceIterator};
use super::super::session::Session;
use super::super::utils::Query;
use super::base::V2API;
use super::protocol;


/// Structure representing a key pair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    session: Rc<Session>,
    inner: protocol::KeyPair
}

/// A query to server list.
#[derive(Clone, Debug)]
pub struct KeyPairQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
}

/// A request to create a key pair.
#[derive(Clone, Debug)]
pub struct NewKeyPair {
    session: Rc<Session>,
    name: String,
    public_key: Option<String>,
}


impl KeyPair {
    /// Load a KeyPair object.
    pub(crate) fn new<Id: AsRef<str>>(session: Rc<Session>, id: Id)
            -> Result<KeyPair> {
        let inner = session.get_keypair(id)?;
        Ok(KeyPair {
            session: session,
            inner: inner
        })
    }

    /// Delete the key pair.
    pub fn delete(self) -> Result<()> {
        self.session.delete_keypair(&self.inner.name)
    }

    transparent_property! {
        #[doc = "Key pair fingerprint."]
        fingerprint: ref String
    }

    transparent_property! {
        #[doc = "Key pair type, if available."]
        key_type: Option<protocol::KeyPairType>
    }

    transparent_property! {
        #[doc = "Key pair name."]
        name: ref String
    }
}

impl Refresh for KeyPair {
    /// Refresh the keypair.
    fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_keypair(&self.inner.name)?;
        Ok(())
    }
}

impl KeyPairQuery {
    pub(crate) fn new(session: Rc<Session>) -> KeyPairQuery {
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
    pub fn into_iter(self) -> ResourceIterator<KeyPair> {
        debug!("Fetching key pairs with {:?}", self.query);
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<KeyPair>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<KeyPair> {
        debug!("Fetching one key pair with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl NewKeyPair {
    /// Start creating a key pair.
    pub(crate) fn new(session: Rc<Session>, name: String)
            -> NewKeyPair {
        NewKeyPair {
            session: session,
            name: name,
            public_key: None,
        }
    }

    /// Request creation of a key pair.
    ///
    /// This call fails immediately if no public_key is provided.
    pub fn create(self) -> Result<KeyPair> {
        let request = if let Some(public_key) = self.public_key {
            protocol::KeyPairCreate {
                key_type: None,  // TODO
                name: self.name,
                public_key: public_key
            }
        } else {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Public key contents is required"));
        };

        let keypair = self.session.create_keypair(request)?;
        Ok(KeyPair {
            session: self.session,
            inner: keypair
        })
    }

    /// Add public key from a reader.
    pub fn from_reader<R>(self, reader: &mut R) -> io::Result<NewKeyPair> where R: io::Read {
        let mut s = String::new();
        let _ = reader.read_to_string(&mut s)?;
        Ok(self.from_string(s))
    }

    /// Add public key from a string.
    pub fn from_string<S>(mut self, public_key: S) -> NewKeyPair where S: Into<String> {
        self.set_string(public_key);
        self
    }

    /// Add public key from a string.
    pub fn set_string<S>(&mut self, public_key: S) where S: Into<String> {
        self.public_key = Some(public_key.into());
    }
}

impl ResourceId for KeyPair {
    fn resource_id(&self) -> String {
        self.name().clone()
    }
}

impl ListResources for KeyPair {
    const DEFAULT_LIMIT: usize = 50;

    fn can_paginate(session: &Session) -> Result<bool> {
        session.supports_keypair_pagination()
    }

    fn list_resources<Q: Serialize + Debug>(session: Rc<Session>, query: Q)
            -> Result<Vec<KeyPair>> {
        Ok(session.list_keypairs(&query)?.into_iter().map(|item| KeyPair {
            session: session.clone(),
            inner: item
        }).collect())
    }
}

impl IntoFallibleIterator for KeyPairQuery {
    type Item = KeyPair;

    type Error = Error;

    type IntoIter = ResourceIterator<KeyPair>;

    fn into_fallible_iterator(self) -> ResourceIterator<KeyPair> {
        self.into_iter()
    }
}

impl From<KeyPair> for KeyPairRef {
    fn from(value: KeyPair) -> KeyPairRef {
        KeyPairRef::new_verified(value.inner.name)
    }
}

impl KeyPairRef {
    /// Verify this reference and convert to an ID, if possible.
    #[cfg(feature = "compute")]
    pub(crate) fn into_verified(self, session: &Session) -> Result<String> {
        Ok(if self.verified {
            self.value
        } else {
            session.get_keypair(&self.value)?.name
        })
    }
}
