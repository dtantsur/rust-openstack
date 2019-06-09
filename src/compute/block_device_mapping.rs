// Copyright 2018-2019 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Block device mapping for the Compute API.

use super::super::common;
use super::super::session::Session;
use super::super::Result;

use serde::ser::{Serialize, SerializeStruct, Serializer};

protocol_enum! {
    #[doc = "A destination type for a block device."]
    enum BlockDeviceDestinationType {
        #[doc = "Local ephemeral device."]
        Local = "local",

        #[doc = "Attached remote volume."]
        Volume = "volume"
    }
}

/// A source of a block device.
#[derive(Clone, Debug)]
pub enum BlockDeviceSource {
    /// A device from an image.
    Image(common::ImageRef),

    /// A device from a volume.
    Volume(common::VolumeRef),

    /// A device from a snapshot.
    Snapshot(common::SnapshotRef),
}

impl BlockDeviceSource {
    #[inline]
    fn source_type(&self) -> &'static str {
        match self {
            BlockDeviceSource::Image(..) => "image",
            BlockDeviceSource::Volume(..) => "volume",
            BlockDeviceSource::Snapshot(..) => "snapshot",
        }
    }

    #[inline]
    fn uuid(&self) -> &str {
        match self {
            BlockDeviceSource::Image(image) => image.as_ref(),
            BlockDeviceSource::Volume(volume) => volume.as_ref(),
            BlockDeviceSource::Snapshot(snapshot) => snapshot.as_ref(),
        }
    }
}

impl common::IntoVerified for BlockDeviceSource {
    fn into_verified(self, session: &Session) -> Result<Self> {
        Ok(match self {
            BlockDeviceSource::Image(inner) => {
                BlockDeviceSource::Image(inner.into_verified(session)?)
            }
            BlockDeviceSource::Volume(inner) => {
                BlockDeviceSource::Volume(inner.into_verified(session)?)
            }
            BlockDeviceSource::Snapshot(inner) => {
                BlockDeviceSource::Snapshot(inner.into_verified(session)?)
            }
        })
    }
}

/// A block device to attach to a server.
#[derive(Clone, Debug)]
pub struct BlockDevice {
    /// Boot index of the device if it's intended to be bootable.
    ///
    /// # Note
    ///
    /// Not all backends support values other than `None` and `Some(0)`.
    pub boot_index: Option<u16>,

    /// Whether to delete the created volume on termination (defaults to `false`).
    pub delete_on_termination: bool,

    /// A type of the destination: local disk or persistent volume.
    pub destination_type: BlockDeviceDestinationType,

    /// Format of the target device if it needs to be formatted.
    pub guest_format: Option<String>,

    /// The size (in GiB) of the created volume (if any).
    ///
    /// # Note
    ///
    /// This is only mandatory when creating `source` is `None`.
    pub size_gib: Option<u32>,

    /// A source for this block device (if any).
    pub source: Option<BlockDeviceSource>,

    // Do not create directly, will be extended in the future.
    __nonexhaustive: (),
}

impl BlockDevice {
    /// Create a block device from the specified source.
    pub fn new(
        source: BlockDeviceSource,
        destination_type: BlockDeviceDestinationType,
    ) -> BlockDevice {
        BlockDevice {
            boot_index: None,
            delete_on_termination: false,
            destination_type,
            guest_format: None,
            size_gib: None,
            source: Some(source),
            __nonexhaustive: (),
        }
    }

    /// Create a swap device.
    pub fn swap(size_gib: u32) -> BlockDevice {
        BlockDevice {
            boot_index: None,
            delete_on_termination: false,
            destination_type: BlockDeviceDestinationType::Local,
            guest_format: Some("swap".into()),
            size_gib: Some(size_gib),
            source: None,
            __nonexhaustive: (),
        }
    }

    /// Attach an image.
    ///
    /// This is used for the entry referring to the image that the instance is being booted with.
    /// Boot index 0 is used for it.
    ///
    /// Use `from_new_volume` to create a volume from any image.
    pub fn from_image<I>(image: I) -> BlockDevice
    where
        I: Into<common::ImageRef>,
    {
        BlockDevice {
            boot_index: Some(0),
            delete_on_termination: false,
            destination_type: BlockDeviceDestinationType::Local,
            guest_format: None,
            size_gib: None,
            source: Some(BlockDeviceSource::Image(image.into())),
            __nonexhaustive: (),
        }
    }

    /// Attach a remote volume.
    ///
    /// The volume will be the first bootable device if `is_boot_device` is `true`.
    pub fn from_volume<V>(volume: V, is_boot_device: bool) -> BlockDevice
    where
        V: Into<common::VolumeRef>,
    {
        BlockDevice {
            boot_index: if is_boot_device { Some(0) } else { None },
            delete_on_termination: false,
            destination_type: BlockDeviceDestinationType::Volume,
            guest_format: None,
            size_gib: None,
            source: Some(BlockDeviceSource::Volume(volume.into())),
            __nonexhaustive: (),
        }
    }

    /// Create a new empty volume.
    pub fn from_empty_volume(size_gib: u32) -> BlockDevice {
        BlockDevice {
            boot_index: None,
            delete_on_termination: false,
            destination_type: BlockDeviceDestinationType::Volume,
            guest_format: None,
            size_gib: Some(size_gib),
            source: None,
            __nonexhaustive: (),
        }
    }

    /// Create a volume from an image.
    ///
    /// The volume will be the first bootable device if `is_boot_device` is `true`.
    pub fn from_new_volume<I>(image: I, size_gib: u32, is_boot_device: bool) -> BlockDevice
    where
        I: Into<common::ImageRef>,
    {
        BlockDevice {
            boot_index: if is_boot_device { Some(0) } else { None },
            delete_on_termination: false,
            destination_type: BlockDeviceDestinationType::Volume,
            guest_format: None,
            size_gib: Some(size_gib),
            source: Some(BlockDeviceSource::Image(image.into())),
            __nonexhaustive: (),
        }
    }

    #[inline]
    fn non_null_field_count(&self) -> usize {
        let mut count = 4;
        if self.source.is_some() {
            count += 1;
        }
        if self.guest_format.is_some() {
            count += 1;
        }
        if self.size_gib.is_some() {
            count += 1
        }
        count
    }
}

impl common::IntoVerified for BlockDevice {
    fn into_verified(self, session: &Session) -> Result<Self> {
        Ok(if let Some(source) = self.source {
            BlockDevice {
                source: Some(source.into_verified(session)?),
                ..self
            }
        } else {
            // No source - nothing to verify.
            self
        })
    }
}

impl common::IntoVerified for Vec<BlockDevice> {
    fn into_verified(self, session: &Session) -> Result<Self> {
        let mut result = Vec::with_capacity(self.len());
        for item in self {
            result.push(item.into_verified(session)?);
        }
        Ok(result)
    }
}

impl Serialize for BlockDevice {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bd = serializer.serialize_struct("BlockDevice", self.non_null_field_count())?;
        bd.serialize_field("boot_index", &self.boot_index)?;
        bd.serialize_field("delete_on_termination", &self.delete_on_termination)?;
        bd.serialize_field("destination_type", &self.destination_type)?;
        if let Some(ref guest_format) = self.guest_format {
            bd.serialize_field("guest_format", guest_format)?;
        }
        if let Some(ref source) = self.source {
            bd.serialize_field("source_type", source.source_type())?;
            bd.serialize_field("uuid", source.uuid())?;
        } else {
            bd.serialize_field("source_type", "blank")?;
        }
        if let Some(volume_size) = self.size_gib {
            bd.serialize_field("volume_size", &volume_size)?;
        }
        bd.end()
    }
}
