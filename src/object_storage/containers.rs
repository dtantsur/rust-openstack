// Copyright 2019 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Containers of objects.

use async_trait::async_trait;
use futures::{pin_mut, Stream, TryStreamExt};

use super::super::common::{ContainerRef, IntoVerified, Refresh};
use super::super::session::Session;
use super::super::utils::{try_one, Query};
use super::super::Result;
use super::objects::{Object, ObjectQuery};
use super::{api, protocol};

/// A query to containers.
#[derive(Clone, Debug)]
pub struct ContainerQuery {
    session: Session,
    query: Query,
    limit: Option<usize>,
    marker: Option<String>,
}

/// Structure representing a single container.
#[derive(Clone, Debug)]
pub struct Container {
    session: Session,
    inner: protocol::Container,
}

impl Container {
    /// Create a new Container object.
    pub(crate) fn new(session: Session, inner: protocol::Container) -> Container {
        Container { session, inner }
    }

    /// Create a new container.
    pub(crate) async fn create<Id: AsRef<str>>(session: Session, name: Id) -> Result<Container> {
        let c_id = name.as_ref();
        let _ = api::create_container(&session, c_id).await?;
        let inner = api::get_container(&session, c_id).await?;
        Ok(Container::new(session, inner))
    }

    /// Load a Container object.
    pub(crate) async fn load<Id: AsRef<str>>(session: Session, name: Id) -> Result<Container> {
        let inner = api::get_container(&session, name).await?;
        Ok(Container::new(session, inner))
    }

    /// Delete the container.
    ///
    /// If `delete_objects` is `true`, all objects inside the container are deleted first.
    /// Otherwise deletion will fail if the container is non-empty.
    pub async fn delete(self, delete_objects: bool) -> Result<()> {
        if delete_objects {
            let mut iter = self.find_objects().into_stream().await?;
            pin_mut!(iter);
            while let Some(obj) = iter.try_next().await? {
                obj.delete().await?;
            }
        }
        api::delete_container(&self.session, self.inner.name).await
    }

    /// Find objects inside this container.
    ///
    /// Returns a query.
    #[inline]
    pub fn find_objects(&self) -> ObjectQuery {
        ObjectQuery::new(self.session.clone(), self.inner.name.clone())
    }

    /// List all objects inside this container.
    #[inline]
    pub async fn list_objects(&self) -> Result<Vec<Object>> {
        self.find_objects().all().await
    }

    transparent_property! {
        #[doc = "Total size of the container."]
        bytes: u64
    }

    transparent_property! {
        #[doc = "Container name."]
        name: ref String
    }

    transparent_property! {
        #[doc = "Number of objects in the container."]
        object_count: u64
    }
}

#[async_trait]
impl Refresh for Container {
    /// Refresh the container.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_container(&self.session, &self.inner.name).await?;
        Ok(())
    }
}

impl ContainerQuery {
    pub(crate) fn new(session: Session) -> ContainerQuery {
        ContainerQuery {
            session,
            query: Query::new(),
            limit: None,
            marker: None,
        }
    }

    /// Add marker to the request.
    ///
    /// Using this disables automatic pagination.
    pub fn with_marker<T: Into<String>>(mut self, marker: T) -> Self {
        self.marker = Some(marker.into());
        self
    }

    /// Add limit to the request.
    ///
    /// Using this disables automatic pagination.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    query_filter! {
        #[doc = "Filter by prefix."]
        with_prefix -> prefix
    }

    /// Convert this query into a stream of containers.
    pub async fn into_stream(self) -> Result<impl Stream<Item = Result<Container>>> {
        debug!(
            "Fetching containers with {:?}",
            self.query
        );
        Ok(
            api::list_containers(&self.session, self.query, self.limit, self.marker)
                .await?
                .map_ok(|c| Container::new(self.session.clone(), c)),
        )
    }

    /// Execute this request and return all results.
    pub async fn all(self) -> Result<Vec<Container>> {
        self.into_stream().await?.try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<Container> {
        debug!("Fetching one container with {:?}", self.query);
        // We need only one result. We fetch maximum two to be able
        // to check if the query yieled more than one result.
        self.limit = Some(2);
        try_one(self.into_stream().await?).await
    }
}

impl From<Container> for ContainerRef {
    fn from(value: Container) -> ContainerRef {
        ContainerRef::new_verified(value.inner.name)
    }
}

#[cfg(feature = "object-storage")]
impl IntoVerified for ContainerRef {}
