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

//! JSON structures and protocol bits for the Block Storage API.

#![allow(non_snake_case)]
#![allow(missing_docs)]

use serde::Deserialize;

// use super::super::common;

protocol_enum! {
    #[doc = "Possible volume statuses."]
    enum VolumeStatus {
        Creating = "creating",
        Available = "available",
        Reserved = "reserved",
        Attaching = "attaching",
        Detaching = "detaching",
        InUse = "in-use",
        Maintenance = "maintenance",
        Deleting = "deleting",
        AwaitingTransfer = "awaiting-transfer",
        Error = "error",
        ErrorDeleting = "error_deleting",
        BackingUp = "backing-up",
        RestoringBackup = "restoring-backup",
        ErrorBackingUp = "error_backing-up",
        ErrorRestoring = "error_restoring",
        ErrorExtending = "error_extending",
        Downloading = "downloading",
        Uploading = "uploading",
        Retyping = "retyping",
        Extending = "extending"
    }
}

protocol_enum! {
    #[doc = "Available sort keys."]
    enum VolumeSortKey {
        CreatedAt = "created_at",
        Id = "id",
        Name = "name",
        UpdatedAt = "updated_at"
    }
}

impl Default for VolumeSortKey {
    fn default() -> VolumeSortKey {
        VolumeSortKey::CreatedAt
    }
}

/// A volume.
#[derive(Debug, Clone, Deserialize)]
pub struct Volume {
    pub id: String,
    pub name: String,
    pub status: VolumeStatus,
}

/// A volume root.
#[derive(Clone, Debug, Deserialize)]
pub struct VolumeRoot {
    pub volume: Volume,
}

/// A list of volumes.
#[derive(Debug, Clone, Deserialize)]
pub struct VolumesRoot {
    pub volumes: Vec<Volume>,
}
