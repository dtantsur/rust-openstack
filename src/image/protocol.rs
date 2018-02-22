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

//! JSON structures and protocol bits for the Image API.

#![allow(non_snake_case)]
#![allow(missing_docs)]

use chrono::{DateTime, FixedOffset};
use reqwest::Url;

use super::super::utils;


protocol_enum! {
    #[doc = "Possible image statuses."]
    enum ImageStatus {
        Queued = "queued",
        Saving = "saving",
        Active = "active",
        Killed = "killed",
        Deleted = "deleted",
        PendingDelete = "pending_delete",
        Deactivated = "deactivated"
    }
}

protocol_enum! {
    #[doc = "Possible image visibility values."]
    enum ImageVisibility {
        Public = "public",
        Community = "community",
        Shared = "shared",
        Private = "private"
    }
}

protocol_enum! {
    #[doc = "Possible container formats."]
    enum ImageContainerFormat {
        AMI = "ami",
        ARI = "ari",
        AKI = "aki",
        Bare = "bare",
        OVF = "ovf",
        OVA = "ova",
        Docker = "docker"
    }
}

protocol_enum! {
    #[doc = "Possible disk formats."]
    enum ImageDiskFormat {
        AMI = "ami",
        ARI = "ari",
        AKI = "aki",
        VHD = "vhd",
        VHDX = "vhdx",
        VMDK = "vmdk",
        Raw = "raw",
        QCOW2 = "qcow2",
        VDI = "vdi",
        ISO = "iso",
        Ploop = "ploop"
    }
}


/// An image.
#[derive(Clone, Deserialize)]
pub struct Image {
    pub architecture: Option<String>,
    pub checksum: Option<String>,
    pub container_format: Option<ImageContainerFormat>,
    pub created_at: DateTime<FixedOffset>,
    #[serde(deserialize_with = "utils::deser_optional_url")]
    pub direct_url: Option<Url>,
    pub disk_format: Option<ImageDiskFormat>,
    pub id: String,
    pub name: String,
    pub size: Option<u64>,
    pub status: ImageStatus,
    pub updated_at: DateTime<FixedOffset>,
    pub virtual_size: Option<u64>,
    pub visibility: Option<ImageVisibility>
}
