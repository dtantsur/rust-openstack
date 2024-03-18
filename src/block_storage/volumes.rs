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
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use super::super::common::{Refresh, ResourceIterator, ResourceQuery};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::waiter::DeletionWaiter;
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

/// A request to create a volume.
#[derive(Clone, Debug)]
pub struct NewVolume {
    session: Session,
    inner: protocol::VolumeCreate,
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
        #[doc = "Migration status."]
        migration_status: ref Option<String>
    }

    transparent_property! {
        #[doc = "Volume attachments."]
        attachments: ref Vec<protocol::VolumeAttachment>
    }

    transparent_property! {
        #[doc = "Volume links."]
        links: ref Vec<protocol::Link>
    }

    transparent_property! {
        #[doc = "Name of the availability zone."]
        availability_zone: ref Option<String>
    }

    transparent_property! {
        #[doc = "Current backend of the volume."]
        host: ref Option<String>
    }

    transparent_property! {
        #[doc = "Whether the volume is encrypted."]
        encrypted: bool
    }

    transparent_property! {
        #[doc = "UUID of the encryption key."]
        encryption_key_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "When the volume was last updated."]
        updated_at: ref Option<String>
    }

    transparent_property! {
        #[doc = "Volume replication status."]
        replication_status: ref Option<String>
    }

    transparent_property! {
        #[doc = "UUID of the snapshot the volume originated from."]
        snapshot_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "UUID of the volume."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Size of the volume in GiB."]
        size: u64
    }

    transparent_property! {
        #[doc = "UUID of the user."]
        user_id: ref String
    }

    transparent_property! {
        #[doc = "UUID of the project."]
        tenant_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "Migration status."]
        migstat: ref Option<String>
    }

    transparent_property! {
        #[doc = "Metadata of the volume."]
        metadata: ref HashMap<String, String>
    }

    transparent_property! {
        #[doc = "Status of the volume."]
        status: protocol::VolumeStatus
    }

    transparent_property! {
        #[doc = "Metadata of the image used to create the volume."]
        image_metadata: ref Option<HashMap<String, String>>
    }

    transparent_property! {
        #[doc = "Description of the volume."]
        description: ref Option<String>
    }

    transparent_property! {
        #[doc = "Whether the volume is multi-attachable."]
        multi_attachable: bool
    }

    transparent_property! {
        #[doc = "UUID of the volume this one originated from."]
        source_volume_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "UUID of the consistency group."]
        consistency_group_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "UUID of the volume that this volume name on the backend is based on."]
        name_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "Name of the volume."]
        name: ref String
    }

    transparent_property! {
        #[doc = "Whether the volume is bootable."]
        bootable: ref String
    }

    transparent_property! {
        #[doc = "When the volume was created."]
        created_at: ref String
    }

    transparent_property! {
        #[doc = "A list of volume objects."]
        volumes: ref Option<Vec<protocol::Volume>>
    }

    transparent_property! {
        #[doc = "Name of the volume type."]
        volume_type: ref String
    }

    transparent_property! {
        #[doc = "UUID of the volume type."]
        volume_type_id: ref Option<HashMap<String, String>>
    }

    transparent_property! {
        #[doc = "UUID of the group."]
        group_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "A list of volume links."]
        volumes_links: ref Option<Vec<String>>
    }

    transparent_property! {
        #[doc = "UUID of the provider for the volume."]
        provider_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "UUID of the service the volume is served on."]
        service_id: ref Option<String>
    }

    transparent_property! {
        #[doc = "Whether the volume has shared targets."]
        shared_targets: Option<bool>
    }

    transparent_property! {
        #[doc = "Cluster name of the volume backend."]
        cluster_name: ref Option<String>
    }

    transparent_property! {
        #[doc = "Whether the volume consumes quota."]
        consumes_quota: Option<bool>
    }

    transparent_property! {
        #[doc = "Total count of volumes requested before pagination."]
        count: Option<u64>
    }

    /// Delete the volume.
    pub async fn delete(self) -> Result<DeletionWaiter<Volume>> {
        api::delete_volume(&self.session, &self.inner.id).await?;
        Ok(DeletionWaiter::new(
            self,
            Duration::new(120, 0),
            Duration::new(1, 0),
        ))
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

impl NewVolume {
    /// Start creating a volume.
    pub(crate) fn new(session: Session, size: u64) -> NewVolume {
        NewVolume {
            session,
            inner: protocol::VolumeCreate::new(size),
        }
    }

    /// Request creation of the volume.
    pub async fn create(self) -> Result<Volume> {
        let inner = api::create_volume(&self.session, self.inner).await?;
        Ok(Volume {
            session: self.session,
            inner,
        })
    }

    creation_inner_field! {
        #[doc = "Set the availability zone."]
        set_availability_zone, with_availability_zone -> availability_zone: optional String
    }

    creation_inner_field! {
        #[doc = "Set the source volume ID."]
        set_source_volume_id, with_source_volume_id -> source_volume_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set the description."]
        set_description, with_description -> description: optional String
    }

    creation_inner_field! {
        #[doc = "Set the snapshot ID."]
        set_snapshot_id, with_snapshot_id -> snapshot_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set the backup ID."]
        set_backup_id, with_backup_id -> backup_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set the name."]
        set_name, with_name -> name: String
    }

    creation_inner_field! {
        #[doc = "Set the image ID."]
        set_image_id, with_image_id -> image_id: optional String
    }

    creation_inner_field! {
        #[doc = "Set the volume type."]
        set_volume_type, with_volume_type -> volume_type: optional String
    }

    creation_inner_field! {
        #[doc = "Set the metadata."]
        set_metadata, with_metadata -> metadata: optional HashMap<String, String>
    }

    creation_inner_field! {
        #[doc = "Set the consistency group ID."]
        set_consistency_group_id, with_consistency_group_id -> consistency_group_id: optional String
    }
}
