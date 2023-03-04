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

use async_trait::async_trait;
use futures::stream::{Stream, TryStreamExt};

use super::super::common::{IntoVerified, KeyPairRef, Refresh, ResourceIterator, ResourceQuery};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, ErrorKind, Result};
use super::{api, protocol};

/// Structure representing a key pair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    session: Session,
    inner: protocol::KeyPair,
}

/// A query to server list.
#[derive(Clone, Debug)]
pub struct KeyPairQuery {
    session: Session,
    query: Query,
    can_paginate: bool,
}

/// A request to create a key pair.
#[derive(Clone, Debug)]
pub struct NewKeyPair {
    session: Session,
    inner: protocol::KeyPairCreate,
}

impl KeyPair {
    /// Load a KeyPair object.
    pub(crate) async fn new<Id: AsRef<str>>(session: Session, id: Id) -> Result<KeyPair> {
        let inner = api::get_keypair(&session, id).await?;
        Ok(KeyPair { session, inner })
    }

    /// Delete the key pair.
    pub async fn delete(self) -> Result<()> {
        api::delete_keypair(&self.session, &self.inner.name).await
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

#[async_trait]
impl Refresh for KeyPair {
    /// Refresh the keypair.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_keypair(&self.session, &self.inner.name).await?;
        Ok(())
    }
}

impl KeyPairQuery {
    pub(crate) fn new(session: Session) -> KeyPairQuery {
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

    /// Convert this query into a stream executing the request.
    ///
    /// Returns a `TryStream`, which is a stream with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_stream(self) -> impl Stream<Item = Result<KeyPair>> {
        debug!("Fetching key pairs with {:?}", self.query);
        ResourceIterator::new(self).into_stream()
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_stream().try_collect().await`.
    pub async fn all(self) -> Result<Vec<KeyPair>> {
        self.into_stream().try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<KeyPair> {
        debug!("Fetching one key pair with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        ResourceIterator::new(self).one().await
    }
}

impl NewKeyPair {
    /// Start creating a key pair.
    pub(crate) fn new(session: Session, name: String) -> NewKeyPair {
        NewKeyPair {
            session,
            inner: protocol::KeyPairCreate::new(name),
        }
    }

    /// Request creation of a key pair.
    ///
    /// This call fails immediately if no public_key is provided.
    pub async fn create(self) -> Result<KeyPair> {
        if self.inner.public_key.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Public key contents is required",
            ));
        };

        let keypair = api::create_keypair(&self.session, self.inner).await?;
        Ok(KeyPair {
            session: self.session,
            inner: keypair,
        })
    }

    /// Create a key pair, generating its public key.
    ///
    /// Returns a new key pair and its private key.
    pub async fn generate(mut self) -> Result<(KeyPair, String)> {
        self.inner.public_key = None;

        let mut keypair = api::create_keypair(&self.session, self.inner).await?;
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
}

#[async_trait]
impl ResourceQuery for KeyPairQuery {
    type Item = KeyPair;

    const DEFAULT_LIMIT: usize = 50;

    async fn can_paginate(&self) -> Result<bool> {
        if self.can_paginate {
            api::supports_keypair_pagination(&self.session).await
        } else {
            Ok(false)
        }
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.name().clone()
    }

    async fn fetch_chunk(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_keypairs(&self.session, &query)
            .await?
            .into_iter()
            .map(|item| KeyPair {
                session: self.session.clone(),
                inner: item,
            })
            .collect())
    }
}

impl From<KeyPair> for KeyPairRef {
    fn from(value: KeyPair) -> KeyPairRef {
        KeyPairRef::new_verified(value.inner.name)
    }
}

#[cfg(feature = "compute")]
#[async_trait]
impl IntoVerified for KeyPairRef {
    /// Verify this reference and convert to an ID, if possible.
    async fn into_verified(self, session: &Session) -> Result<KeyPairRef> {
        Ok(if self.verified {
            self
        } else {
            KeyPairRef::new_verified(api::get_keypair(session, &self.value).await?.name)
        })
    }
}
