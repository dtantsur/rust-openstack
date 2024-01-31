// Copyright 2024 Sandro-Alessio Gierens <sandro@gierens.de>
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

//! Volume management via Block Storage API.

use async_trait::async_trait;
use futures::stream::{Stream, TryStreamExt};
use std::fmt::{self, Display, Formatter};

use super::super::common::{Refresh, ResourceIterator, ResourceQuery};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Result, Sort};
use super::{api, protocol};

/// A query to volume list.
#[derive(Clone, Debug)]
pub struct VolumeQuery {
    session: Session,
    query: Query,
    can_paginate: bool,
    sort: Vec<String>,
}

/// Structure representing a summary of a single volume.
#[derive(Clone, Debug)]
pub struct Volume {
    session: Session,
    inner: protocol::Volume,
}

impl Display for Volume {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self.inner)
    }
}

impl Volume {
    /// Create an Volume object.
    pub(crate) async fn new<Id: AsRef<str>>(session: Session, id: Id) -> Result<Volume> {
        let inner = api::get_volume(&session, id).await?;
        Ok(Volume { session, inner })
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Volume name."]
        name: ref String
    }
}

#[async_trait]
impl Refresh for Volume {
    /// Refresh the volume.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_volume_by_id(&self.session, &self.inner.id).await?;
        Ok(())
    }
}

impl VolumeQuery {
    pub(crate) fn new(session: Session) -> VolumeQuery {
        VolumeQuery {
            session,
            query: Query::new(),
            can_paginate: true,
            sort: Vec::new(),
        }
    }

    /// Add sorting to the request.
    pub fn sort_by(mut self, sort: Sort<protocol::VolumeSortKey>) -> Self {
        let (field, direction) = sort.into();
        self.sort.push(format!("{field}:{direction}"));
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
        #[doc = "Filter by volume name."]
        with_name -> name
    }

    query_filter! {
        #[doc = "Filter by volume status."]
        with_status -> status: protocol::VolumeStatus
    }

    /// Convert this query into a stream executing the request.
    ///
    /// Returns a `TryStream`, which is a stream with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    pub fn into_stream(
        mut self,
    ) -> impl Stream<Item = Result<<VolumeQuery as ResourceQuery>::Item>> {
        if !self.sort.is_empty() {
            self.query.push_str("sort", self.sort.join(","));
        }
        debug!("Fetching volumes with {:?}", self.query);
        ResourceIterator::new(self).into_stream()
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_stream().try_collect().await`.
    pub async fn all(self) -> Result<Vec<Volume>> {
        self.into_stream().try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<Volume> {
        debug!("Fetching one volume with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yields more than one result.
            self.query.push("limit", 2);
        }

        ResourceIterator::new(self).one().await
    }
}

#[async_trait]
impl ResourceQuery for VolumeQuery {
    type Item = Volume;

    const DEFAULT_LIMIT: usize = 50;

    async fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    async fn fetch_chunk(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Result<Vec<Self::Item>> {
        let query = self.query.with_marker_and_limit(limit, marker);
        Ok(api::list_volumes(&self.session, &query)
            .await?
            .into_iter()
            .map(|item| Volume {
                session: self.session.clone(),
                inner: item,
            })
            .collect())
    }
}
