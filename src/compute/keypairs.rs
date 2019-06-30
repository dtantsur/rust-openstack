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

use std::io;
use std::rc::Rc;

use fallible_iterator::{FallibleIterator, IntoFallibleIterator};

use super::super::common::{IntoVerified, KeyPairRef, Refresh, ResourceIterator, ResourceQuery};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, ErrorKind, Result};
use super::{api, protocol};

/// Structure representing a key pair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    session: Rc<Session>,
    inner: protocol::KeyPair,
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
    inner: protocol::KeyPairCreate,
}

impl KeyPair {
    /// Load a KeyPair object.
    pub(crate) fn new<Id: AsRef<str>>(session: Rc<Session>, id: Id) -> Result<KeyPair> {
        let inner = api::get_keypair(&session, id)?;
        Ok(KeyPair { session, inner })
    }

    /// Delete the key pair.
    pub fn delete(self) -> Result<()> {
        api::delete_keypair(&self.session, &self.inner.name)
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
        self.inner = api::get_keypair(&self.session, &self.inner.name)?;
        Ok(())
    }
}

impl KeyPairQuery {
    pub(crate) fn new(session: Rc<Session>) -> KeyPairQuery {
        KeyPairQuery {
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

    /// Convert this query into an iterator executing the request.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(self) -> ResourceIterator<KeyPairQuery> {
        debug!("Fetching key pairs with {:?}", self.query);
        ResourceIterator::new(self)
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
    pub(crate) fn new(session: Rc<Session>, name: String) -> NewKeyPair {
        NewKeyPair {
            session,
            inner: protocol::KeyPairCreate::new(name),
        }
    }

    /// Request creation of a key pair.
    ///
    /// This call fails immediately if no public_key is provided.
    pub fn create(self) -> Result<KeyPair> {
        if self.inner.public_key.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Public key contents is required",
            ));
        };

        let keypair = api::create_keypair(&self.session, self.inner)?;
        Ok(KeyPair {
            session: self.session,
            inner: keypair,
        })
    }

    /// Create a key pair, generating its public key.
    ///
    /// Returns a new key pair and its private key.
    pub fn generate(mut self) -> Result<(KeyPair, String)> {
        self.inner.public_key = None;

        let mut keypair = api::create_keypair(&self.session, self.inner)?;
        if let Some(private_key) = keypair.private_key.take() {
            let result = KeyPair {
                session: self.session,
                inner: keypair,
            };

            Ok((result, private_key))
        } else {
            Err(Error::new(
                ErrorKind::InvalidResponse,
                "Missing private key in the response",
            ))
        }
    }

    creation_inner_field! {
        #[doc = "Set type of the key pair."]
        set_key_type, with_key_type -> key_type: optional protocol::KeyPairType
    }

    creation_inner_field! {
        #[doc = "Set name of the key pair."]
        set_name, with_name -> name: String
    }

    creation_inner_field! {
        #[doc = "Set name of the key pair."]
        set_public_key, with_public_key -> public_key: optional String
    }

    /// Add public key from a reader.
    #[deprecated(since = "0.2.2", note = "Use with_public_key")]
    pub fn from_reader<R>(self, reader: &mut R) -> io::Result<NewKeyPair>
    where
        R: io::Read,
    {
        let mut s = String::new();
        let _ = reader.read_to_string(&mut s)?;
        Ok(self.with_public_key(s))
    }

    /// Add public key from a string.
    #[deprecated(since = "0.2.2", note = "Use with_public_key")]
    pub fn from_string<S>(self, public_key: S) -> NewKeyPair
    where
        S: Into<String>,
    {
        self.with_public_key(public_key)
    }

    /// Add public key from a string.
    #[deprecated(since = "0.2.2", note = "Use set_public_key")]
    pub fn set_string<S>(&mut self, public_key: S)
    where
        S: Into<String>,
    {
        self.set_public_key(public_key);
    }
}

impl ResourceQuery for KeyPairQuery {
    type Item = KeyPair;

    const DEFAULT_LIMIT: usize = 50;

    fn can_paginate(&self) -> Result<bool> {
        if self.can_paginate {
            api::supports_keypair_pagination(&self.session)
        } else {
            Ok(false)
        }
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.name().clone()
    }

    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_keypairs(&self.session, &query)?
            .into_iter()
            .map(|item| KeyPair {
                session: self.session.clone(),
                inner: item,
            })
            .collect())
    }
}

impl IntoFallibleIterator for KeyPairQuery {
    type Item = KeyPair;

    type Error = Error;

    type IntoFallibleIter = ResourceIterator<KeyPairQuery>;

    fn into_fallible_iter(self) -> Self::IntoFallibleIter {
        self.into_iter()
    }
}

impl From<KeyPair> for KeyPairRef {
    fn from(value: KeyPair) -> KeyPairRef {
        KeyPairRef::new_verified(value.inner.name)
    }
}

#[cfg(feature = "compute")]
impl IntoVerified for KeyPairRef {
    /// Verify this reference and convert to an ID, if possible.
    fn into_verified(self, session: &Session) -> Result<KeyPairRef> {
        Ok(if self.verified {
            self
        } else {
            KeyPairRef::new_verified(api::get_keypair(session, &self.value)?.name)
        })
    }
}
