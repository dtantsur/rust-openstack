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

//! Stored objects.

use async_trait::async_trait;
use futures::io::AsyncRead;
use futures::{Stream, TryStreamExt};

use super::super::common::{
    ContainerRef, IntoVerified, ObjectRef, Refresh,
};
use super::super::session::Session;
use super::super::utils::{try_one, Query};
use super::super::Result;
use super::{api, protocol};

/// A query to objects.
#[derive(Clone, Debug)]
pub struct ObjectQuery {
    session: Session,
    c_name: String,
    query: Query,
    limit: Option<usize>,
    marker: Option<String>,
}

/// Structure representing an object.
#[derive(Clone, Debug)]
pub struct Object {
    session: Session,
    inner: protocol::Object,
    c_name: String,
}

impl Object {
    /// Create a new Object object.
    pub(crate) fn new(session: Session, inner: protocol::Object, c_name: String) -> Object {
        Object {
            session,
            inner,
            c_name,
        }
    }

    /// Create an object.
    pub(crate) async fn create<C, Id, R>(
        session: Session,
        container: C,
        name: Id,
        body: R,
    ) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
        R: AsyncRead + Send + Sync + 'static,
    {
        let c_ref = container.into();
        let c_name = c_ref.to_string();
        let inner = api::create_object(&session, c_ref, name, body).await?;
        Ok(Object::new(session, inner, c_name))
    }

    /// Load an Object.
    pub(crate) async fn load<C, Id>(session: Session, container: C, name: Id) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
    {
        let c_ref = container.into();
        let c_name = c_ref.to_string();
        let inner = api::get_object(&session, c_ref, name).await?;
        Ok(Object::new(session, inner, c_name))
    }

    /// Delete the object.
    #[inline]
    pub async fn delete(self) -> Result<()> {
        api::delete_object(&self.session, &self.c_name, self.inner.name).await
    }

    /// Download the object.
    ///
    /// The object can be read from the resulting reader.
    #[inline]
    pub async fn download(&self) -> Result<impl AsyncRead + Send + '_> {
        api::download_object(&self.session, &self.c_name, &self.inner.name).await
    }

    transparent_property! {
        #[doc = "Total size of the object."]
        bytes: u64
    }

    /// Container name.
    #[inline]
    pub fn container_name(&self) -> &String {
        &self.c_name
    }

    transparent_property! {
        #[doc = "Object content type (if set)."]
        content_type: ref Option<String>
    }

    transparent_property! {
        #[doc = "Object name."]
        name: ref String
    }
}

#[async_trait]
impl Refresh for Object {
    /// Refresh the object.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_object(&self.session, &self.c_name, &self.inner.name).await?;
        Ok(())
    }
}

impl ObjectQuery {
    pub(crate) fn new<C: Into<ContainerRef>>(session: Session, container: C) -> ObjectQuery {
        ObjectQuery {
            session,
            c_name: container.into().into(),
            query: Query::new(),
            limit: None,
            marker: None,
        }
    }

    /// Add marker to the request.
    pub fn with_marker<T: Into<String>>(mut self, marker: T) -> Self {
        self.marker = Some(marker.into());
        self
    }

    /// Add limit to the request.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Convert this query into a stream of objects.
    pub async fn into_stream(self) -> Result<impl Stream<Item = Result<Object>>> {
        debug!(
            "Fetching objects in container {} with {:?}",
            self.c_name, self.query
        );
        Ok(api::list_objects(
            &self.session,
            self.c_name,
            self.query,
            self.limit,
            self.marker,
        )
        .await?
        .map_ok(|obj| Object::new(self.session.clone(), obj, self.c_name.clone())))
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub async fn all(self) -> Result<Vec<Object>> {
        self.into_stream().await?.try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<Object> {
        debug!(
            "Fetching one object in container {} with {:?}",
            self.c_name, self.query
        );
        // We need only one result. We fetch maximum two to be able
        // to check if the query yieled more than one result.
        self.limit = Some(2);
        try_one(self.into_stream().await?).await
    }
}

impl From<Object> for ObjectRef {
    fn from(value: Object) -> ObjectRef {
        ObjectRef::new_verified(value.inner.name)
    }
}

#[cfg(feature = "object-storage")]
impl IntoVerified for ObjectRef {}
