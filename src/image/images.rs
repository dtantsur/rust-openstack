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

use std::rc::Rc;

use chrono::{DateTime, FixedOffset};
use fallible_iterator::{FallibleIterator, IntoFallibleIterator};

use super::super::common::{ImageRef, IntoVerified, Refresh, ResourceIterator, ResourceQuery};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Error, Result, Sort};
use super::{api, protocol};

/// A query to image list.
#[derive(Clone, Debug)]
pub struct ImageQuery {
    session: Rc<Session>,
    query: Query,
    can_paginate: bool,
    sort: Vec<String>,
}

/// Structure representing a single image.
#[derive(Clone, Debug)]
pub struct Image {
    session: Rc<Session>,
    inner: protocol::Image,
}

impl Image {
    /// Create an Image object.
    pub(crate) fn new<Id: AsRef<str>>(session: Rc<Session>, id: Id) -> Result<Image> {
        let inner = api::get_image(&session, id)?;
        Ok(Image { session, inner })
    }

    transparent_property! {
        #[doc = "Image architecture."]
        architecture: ref Option<String>
    }

    transparent_property! {
        #[doc = "Checksum of the image."]
        checksum: ref Option<String>
    }

    transparent_property! {
        #[doc = "Container format."]
        container_format: Option<protocol::ImageContainerFormat>
    }

    transparent_property! {
        #[doc = "Creating date and time."]
        created_at: DateTime<FixedOffset>
    }

    transparent_property! {
        #[doc = "Disk format."]
        disk_format: Option<protocol::ImageDiskFormat>
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    /// Minimum required disk size in GiB.
    ///
    /// Can be zero, if no requirements are known.
    pub fn minimum_required_disk(&self) -> u32 {
        self.inner.min_disk
    }

    /// Minimum required disk size in GiB, if set.
    ///
    /// Can be zero, if no requirements are known.
    pub fn minimum_required_ram(&self) -> u32 {
        self.inner.min_ram
    }

    transparent_property! {
        #[doc = "Image name."]
        name: ref String
    }

    transparent_property! {
        #[doc = "Image size in bytes."]
        size: Option<u64>
    }

    transparent_property! {
        #[doc = "Image status."]
        status: protocol::ImageStatus
    }

    transparent_property! {
        #[doc = "Last update date and time."]
        updated_at: DateTime<FixedOffset>
    }

    transparent_property! {
        #[doc = "Virtual size of the image."]
        virtual_size: Option<u64>
    }

    transparent_property! {
        #[doc = "Image visibility."]
        visibility: protocol::ImageVisibility
    }
}

impl Refresh for Image {
    /// Refresh the image.
    fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_image_by_id(&self.session, &self.inner.id)?;
        Ok(())
    }
}

impl ImageQuery {
    pub(crate) fn new(session: Rc<Session>) -> ImageQuery {
        ImageQuery {
            session,
            query: Query::new(),
            can_paginate: true,
            sort: Vec::new(),
        }
    }

    /// Add sorting to the request.
    pub fn sort_by(mut self, sort: Sort<protocol::ImageSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.sort.push(format!("{}:{}", field, direction));
        self
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

    query_filter! {
        #[doc = "Filter by image name."]
        with_name -> name
    }

    query_filter! {
        #[doc = "Filter by image status."]
        with_status -> status: protocol::ImageStatus
    }

    query_filter! {
        #[doc = "Filter by visibility."]
        with_visibility -> visibility: protocol::ImageVisibility
    }

    /// Convert this query into an iterator executing the request.
    ///
    /// Returns a `FallibleIterator`, which is an iterator with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_iter(mut self) -> ResourceIterator<ImageQuery> {
        if !self.sort.is_empty() {
            self.query.push_str("sort", self.sort.join(","));
        }
        debug!("Fetching images with {:?}", self.query);
        ResourceIterator::new(self)
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_iter().collect()`.
    pub fn all(self) -> Result<Vec<Image>> {
        self.into_iter().collect()
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub fn one(mut self) -> Result<Image> {
        debug!("Fetching one image with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push("limit", 2);
        }

        self.into_iter().one()
    }
}

impl ResourceQuery for ImageQuery {
    type Item = Image;

    const DEFAULT_LIMIT: usize = 50;

    fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_images(&self.session, &query)?
            .into_iter()
            .map(|item| Image {
                session: self.session.clone(),
                inner: item,
            })
            .collect())
    }
}

impl IntoFallibleIterator for ImageQuery {
    type Item = Image;

    type Error = Error;

    type IntoFallibleIter = ResourceIterator<ImageQuery>;

    fn into_fallible_iter(self) -> Self::IntoFallibleIter {
        self.into_iter()
    }
}

impl From<Image> for ImageRef {
    fn from(value: Image) -> ImageRef {
        ImageRef::new_verified(value.inner.id)
    }
}

#[cfg(feature = "image")]
impl IntoVerified for ImageRef {
    /// Verify this reference and convert to an ID, if possible.
    fn into_verified(self, session: &Session) -> Result<ImageRef> {
        Ok(if self.verified {
            self
        } else {
            ImageRef::new_verified(api::get_image(session, &self.value)?.id)
        })
    }
}
