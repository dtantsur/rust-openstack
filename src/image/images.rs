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

//! Image management via Image API.

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


/// A query to image list.
#[derive(Clone, Debug)]
pub struct ImageQuery<'session> {
    session: &'session Session,
    query: Query,
    can_paginate: bool,
    sort: Vec<String>
}

/// Structure representing a single image.
#[derive(Clone, Debug)]
pub struct Image<'session> {
    session: &'session Session,
    inner: protocol::Image
}

impl<'session> Image<'session> {
    /// Load a Image object.
    pub(crate) fn new<Id: AsRef<str>>(session: &'session Session, id: Id)
            -> Result<Image<'session>> {
        let inner = session.get_image(id)?;
        Ok(Image {
            session: session,
            inner: inner
        })
    }

    /// Refresh the image.
    pub fn refresh(&mut self) -> Result<()> {
        self.inner = self.session.get_image(&self.inner.id)?;
        Ok(())
    }

    /// Get a reference to creation date and time.
    pub fn created_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.created_at
    }

    /// Get a reference to image unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to image name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get a reference to last update date and time.
    pub fn updated_at(&self) -> &DateTime<FixedOffset> {
        &self.inner.updated_at
    }
}

impl<'session> ImageQuery<'session> {
    pub(crate) fn new(session: &'session Session) -> ImageQuery<'session> {
        ImageQuery {
            session: session,
            query: Query::new(),
            can_paginate: true,
            sort: Vec::new()
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
    pub fn sort_by(mut self, sort: Sort<protocol::ImageSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.sort.push(format!("{}:{}", field, direction));
        self
    }

    /// Filter by image name (a database regular expression).
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
    pub fn into_iter(mut self) -> ResourceIterator<'session, Image<'session>> {
        if ! self.sort.is_empty() {
            self.query.push_str("sort", self.sort.join(","));
        }
        ResourceIterator::new(self.session, self.query)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Image<'session>>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Image<'session>> {
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        utils::fetch_one(self)
    }
}

impl<'session> ResourceId for Image<'session> {
    fn resource_id(&self) -> String {
        self.id().clone()
    }
}

impl<'session> ListResources<'session> for Image<'session> {
    const DEFAULT_LIMIT: usize = 50;

    fn list_resources<Q: Serialize + Debug>(session: &'session Session, query: Q)
            -> Result<Vec<Image<'session>>> {
        Ok(session.list_images(&query)?.into_iter().map(|item| Image {
            session: session,
            inner: item
        }).collect())
    }
}

impl<'session> IntoFallibleIterator for ImageQuery<'session> {
    type Item = Image<'session>;

    type Error = Error;

    type IntoIter = ResourceIterator<'session, Image<'session>>;

    fn into_fallible_iterator(self) -> ResourceIterator<'session, Image<'session>> {
        self.into_iter()
    }
}

impl<'session> From<Image<'session>> for types::ImageRef {
    fn from(value: Image<'session>) -> types::ImageRef {
        types::ImageRef::from(value.inner.id)
    }
}
