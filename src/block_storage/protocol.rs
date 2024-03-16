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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// A volume attachment.
#[derive(Debug, Clone, Deserialize)]
pub struct VolumeAttachment {
    pub server_id: String, // this should be a reference to a server
    pub attachment_id: String,
    pub attached_at: String,
    pub host_name: String,
    pub volume_id: String, // this should be a reference to a volume
    pub device: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Link {
    pub rel: String,
    pub href: String,
}

/// A volume.
#[derive(Debug, Clone, Deserialize)]
pub struct Volume {
    // TODO: not all fields fully match the API spec:
    // https://docs.openstack.org/api-ref/block-storage/v3/#list-accessible-volumes-with-details
    // Some fields are not actually optional, but don't work without Option<>.
    // Others should maybe be enums, but the possible values are not documented.
    // There are comments for these cases.
    pub migration_status: Option<String>, // consider enum
    pub attachments: Vec<VolumeAttachment>,
    pub links: Vec<Link>,
    pub availability_zone: Option<String>,
    #[serde(rename = "os-vol-host-attr:host")]
    pub host: Option<String>,
    pub encrypted: bool,
    pub encryption_key_id: Option<String>,
    pub updated_at: Option<String>,
    pub replication_status: Option<String>, // not optional in spec, also consider enum
    pub snapshot_id: Option<String>,
    pub id: String,
    pub size: u64,
    pub user_id: String,
    #[serde(rename = "os-vol-tenant-attr:tenant_id")]
    pub tenant_id: Option<String>,
    #[serde(rename = "os-vol-mig-status-attr:migstat")]
    pub migstat: Option<String>, // consider enum
    pub metadata: HashMap<String, String>,
    pub status: VolumeStatus,
    #[serde(rename = "volume_image_metadata")]
    pub image_metadata: Option<HashMap<String, String>>,
    pub description: Option<String>,
    pub multiattach: bool,
    #[serde(rename = "source_volid")]
    pub source_volume_id: Option<String>,
    #[serde(rename = "consistencygroup_id")]
    pub consistency_group_id: Option<String>, // not optional in spec
    #[serde(rename = "os-vol-mig-status-attr:name_id")]
    pub name_id: Option<String>,
    pub name: String,
    pub bootable: String,
    pub created_at: String,
    pub volumes: Option<Vec<Volume>>, // not optional in spec
    pub volume_type: String,          // consider enum
    pub volume_type_id: Option<HashMap<String, String>>, // not optional in spec
    pub group_id: Option<String>,
    pub volumes_links: Option<Vec<String>>,
    pub provider_id: Option<String>,
    pub service_uuid: Option<String>, // not optional in spec
    pub shared_targets: Option<bool>, // not optional in spec
    pub cluster_name: Option<String>,
    pub consumes_quota: Option<bool>,
    pub count: Option<u64>,
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

/// Volume arguments for a create request.
#[derive(Debug, Clone, Serialize)]
pub struct VolumeCreate {
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "source_volid")]
    pub source_volume_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "imageRef")]
    pub image_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "consistency_group_id")]
    pub consistency_group_id: Option<String>,
}

/// A volume create request.
#[derive(Clone, Debug, Serialize)]
pub struct VolumeCreateRoot {
    pub volume: VolumeCreate,
    // NOTE: this can also contain a scheduler_hints field
}

impl VolumeCreate {
    pub fn new(size: u64) -> VolumeCreate {
        VolumeCreate {
            size,
            availability_zone: None,
            source_volume_id: None,
            description: None,
            snapshot_id: None,
            backup_id: None,
            name: None,
            image_id: None,
            volume_type: None,
            metadata: None,
            consistency_group_id: None,
        }
    }
}
