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

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone};
use futures::io::AsyncRead;
use futures::{Stream, TryStreamExt};
use osauth::services::OBJECT_STORAGE;
use reqwest::Url;

use super::super::common::{ContainerRef, IntoVerified, ObjectRef, Refresh};
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

/// A request to create an object.
#[derive(Debug)]
pub struct NewObject<R> {
    session: Session,
    c_name: ContainerRef,
    name: String,
    body: R,
    headers: ObjectHeaders,
}

/// Optional headers for an object.
#[derive(Debug, Default)]
pub struct ObjectHeaders {
    pub delete_after: Option<u32>,
    pub delete_at: Option<i64>,
    pub metadata: HashMap<String, String>,
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
        R: AsyncRead + Sync + Send + 'static,
    {
        let new_object = NewObject::new(
            session,
            container.into(),
            // TODO(dtantsur): get rid of to_string here.
            name.as_ref().to_string(),
            body,
        );
        new_object.create().await
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
        #[doc = "Object hash or ETag, which is a content's md5 hash"]
        hash: ref Option<String>
    }

    transparent_property! {
        #[doc = "Object name."]
        name: ref String
    }

    /// Object url.
    #[inline]
    pub async fn url(&self) -> Result<Url> {
        self.session
            .get_endpoint(OBJECT_STORAGE, &[self.container_name(), self.name()])
            .await
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
            self.c_name.clone(),
            self.query,
            self.limit,
            self.marker,
        )
        .await?
        .map_ok({
            let session = self.session;
            let c_name = self.c_name;
            move |obj| Object::new(session.clone(), obj, c_name.clone())
        }))
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

impl<R: AsyncRead + Sync + Send + 'static> NewObject<R> {
    /// Start creating an object.
    pub(crate) fn new(
        session: Session,
        c_name: ContainerRef,
        name: String,
        body: R,
    ) -> NewObject<R> {
        NewObject {
            session,
            c_name,
            name,
            body,
            headers: ObjectHeaders::default(),
        }
    }

    /// Request creation of the object.
    pub async fn create(self) -> Result<Object> {
        let c_name = self.c_name.clone();

        let inner = api::create_object(
            &self.session,
            self.c_name,
            self.name,
            self.body,
            self.headers,
        )
        .await?;

        Ok(Object::new(self.session, inner, c_name.into()))
    }

    /// Metadata to set on the object.
    #[inline]
    pub fn metadata(&mut self) -> &mut HashMap<String, String> {
        &mut self.headers.metadata
    }

    /// Set TTL in seconds for the object.
    #[inline]
    pub fn with_delete_after(mut self, ttl: u32) -> NewObject<R> {
        self.headers.delete_after = Some(ttl);
        self
    }

    /// Set the date and time when the object must be deleted.
    #[inline]
    pub fn with_delete_at<T: TimeZone>(mut self, datetime: DateTime<T>) -> NewObject<R> {
        self.headers.delete_at = Some(datetime.timestamp());
        self
    }

    /// Insert a new metadata item.
    #[inline]
    pub fn with_metadata<K, V>(mut self, key: K, item: V) -> NewObject<R>
    where
        K: Into<String>,
        V: Into<String>,
    {
        let _ = self.headers.metadata.insert(key.into(), item.into());
        self
    }
}

impl From<Object> for ObjectRef {
    fn from(value: Object) -> ObjectRef {
        ObjectRef::new_verified(value.inner.name)
    }
}

#[cfg(feature = "object-storage")]
impl IntoVerified for ObjectRef {}
