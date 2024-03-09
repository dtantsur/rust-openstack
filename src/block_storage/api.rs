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

//! Foundation bits exposing the Block Storage API.

use std::fmt::Debug;

use osauth::services::BLOCK_STORAGE;
use osauth::ErrorKind;
use serde::Serialize;

use super::super::session::Session;
use super::super::utils;
use super::protocol::*;
use super::super::Result;

/// Delete a volume.
pub async fn delete_volume<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    trace!("Deleting volume {}", id.as_ref());
    let _ = session
        .delete(BLOCK_STORAGE, &["volumes", id.as_ref()])
        .send()
        .await?;
    debug!("Successfully requested deletion of volume {}", id.as_ref());
    Ok(())
}

/// Get an volume.
pub async fn get_volume<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Volume> {
    let s = id_or_name.as_ref();
    match get_volume_by_id(session, s).await {
        Ok(value) => Ok(value),
        Err(err) if err.kind() == ErrorKind::ResourceNotFound => {
            get_volume_by_name(session, s).await
        }
        Err(err) => Err(err),
    }
}

/// Get an volume by its ID.
pub async fn get_volume_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Volume> {
    trace!("Fetching volume {}", id.as_ref());
    let root: VolumeRoot = session.get(BLOCK_STORAGE, &["volumes", id.as_ref()]).fetch().await?;
    trace!("Received {:?}", root.volume);
    Ok(root.volume)
}

/// Get an volume by its name.
pub async fn get_volume_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Volume> {
    trace!("Get volume by name {}", name.as_ref());
    let root: VolumesRoot = session
        .get(BLOCK_STORAGE, &["volumes"])
        .query(&[("name", name.as_ref())])
        .fetch()
        .await?;
    let result = utils::one(
        root.volumes,
        "Volume with given name or ID not found",
        "Too many volumes found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// List volumes.
pub async fn list_volumes<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Volume>> {
    trace!("Listing volumes with {:?}", query);
    let root: VolumesRoot = session.get(BLOCK_STORAGE, &["volumes", "detail"]).query(query).fetch().await?;
    trace!("Received volumes: {:?}", root.volumes);
    Ok(root.volumes)
}
