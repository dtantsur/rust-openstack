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

use std::io::Read;
use std::rc::Rc;
use std::collections::HashMap;

use fallible_iterator::{FallibleIterator, IntoFallibleIterator};

use super::super::common::{
    ContainerRef, IntoVerified, ObjectRef, Refresh, ResourceIterator, ResourceQuery,
};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, Result};
use super::{api, protocol};

/// A query to objects.
#[derive(Clone, Debug)]
pub struct ObjectQuery {
    session: Rc<Session>,
    c_name: String,
    query: Query,
    can_paginate: bool,
}

/// Structure representing an object.
#[derive(Clone, Debug)]
pub struct Object {
    session: Rc<Session>,
    inner: protocol::Object,
    c_name: String,
}

impl Object {
    /// Create a new Object object.
    pub(crate) fn new(session: Rc<Session>, inner: protocol::Object, c_name: String) -> Object {
        Object {
            session,
            inner,
            c_name,
        }
    }

    /// Create an object.
    pub(crate) fn create<C, Id, R>(
        session: Rc<Session>,
        container: C,
        name: Id,
        body: R,
    ) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
        R: Read + Send + 'static,
    {
        let c_ref = container.into();
        let c_name = c_ref.to_string();
        let inner = api::create_object(&session, c_ref, name, body)?;
        Ok(Object::new(session, inner, c_name))
    }

    /// Create an object with headers sent with the request.
    pub(crate) fn create_with_headers<C, Id, R>(
        session: Rc<Session>,
        container: C,
        name: Id,
        body: R,
        headers: HashMap<String, String>,
    ) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
        R: Read + Send + 'static,
    {
        let c_ref = container.into();
        let c_name = c_ref.to_string();
        let inner = api::create_object_with_headers(&session, c_ref, name, body, headers)?;
        Ok(Object::new(session, inner, c_name))
    }

    /// Load an Object.
    pub(crate) fn load<C, Id>(session: Rc<Session>, container: C, name: Id) -> Result<Object>
    where
        C: Into<ContainerRef>,
        Id: AsRef<str>,
    {
        let c_ref = container.into();
        let c_name = c_ref.to_string();
        let inner = api::get_object(&session, c_ref, name)?;
        Ok(Object::new(session, inner, c_name))
    }

    /// Delete the object.
    #[inline]
    pub fn delete(self) -> Result<()> {
        api::delete_object(&self.session, &self.c_name, self.inner.name)
    }

    /// Download the object.
    ///
    /// The object can be read from the resulting reader.
    #[inline]
    pub fn download(&self) -> Result<impl Read + '_> {
        api::download_object(&self.session, &self.c_name, &self.inner.name)
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

impl Refresh for Object {
    /// Refresh the object.
    fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_object(&self.session, &self.c_name, &self.inner.name)?;
        Ok(())
    }
}

impl ObjectQuery {
    pub(crate) fn new<C: Into<ContainerRef>>(session: Rc<Session>, container: C) -> ObjectQuery {
        ObjectQuery {
            session,
            c_name: container.into().into(),
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
    pub fn into_iter(self) -> ResourceIterator<ObjectQuery> {
        debug!(
            "Fetching objects in container {} with {:?}",
            self.c_name, self.query
        );
        ResourceIterator::new(self)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Object>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Object> {
        debug!(
            "Fetching one object in container {} with {:?}",
            self.c_name, self.query
        );
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl ResourceQuery for ObjectQuery {
    type Item = Object;

    const DEFAULT_LIMIT: usize = 100;

    fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.name().clone()
    }

    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_objects(&self.session, &self.c_name, query)?
            .into_iter()
            .map(|item| Object {
                session: self.session.clone(),
                inner: item,
                c_name: self.c_name.clone(),
            })
            .collect())
    }
}

impl IntoFallibleIterator for ObjectQuery {
    type Item = Object;

    type Error = Error;

    type IntoFallibleIter = ResourceIterator<ObjectQuery>;

    fn into_fallible_iter(self) -> Self::IntoFallibleIter {
        self.into_iter()
    }
}

impl From<Object> for ObjectRef {
    fn from(value: Object) -> ObjectRef {
        ObjectRef::new_verified(value.inner.name)
    }
}

#[cfg(feature = "object-storage")]
impl IntoVerified for ObjectRef {}
