// Copyright 2023 Dmitry Tantsur <dtantsur@protonmail.com>
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

use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! info_string_field {

    ($(#[$attr:meta])* $func:ident, $set_func:ident, $with_func:ident -> $const:expr) => {
        $(#[$attr])*
        pub fn $func(&self) -> Option<&String> {
            self.subfield($const)
        }

        $(#[$attr])*
        pub fn $set_func<S: Into<String>>(&mut self, value: S) {
            let _ = self
                .0
                .insert($const.into(), Value::String(value.into()));
        }

        $(#[$attr])*
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    }

}

/// Driver-specific information.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DriverInfo(pub HashMap<String, Value>);

impl Deref for DriverInfo {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DriverInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Common image checksums.
#[derive(Debug, Clone)]
pub enum ImageChecksum<'s> {
    /// MD5 checksum (the default, but not recommended).
    MD5(Cow<'s, str>),
    /// SHA256 checksum.
    SHA256(Cow<'s, str>),
    /// SHA512 checksum.
    SHA512(Cow<'s, str>),
}

impl<'s> ImageChecksum<'s> {
    fn into_type_value(self) -> (&'static str, String) {
        match self {
            ImageChecksum::MD5(s) => ("md5", s.into_owned()),
            ImageChecksum::SHA256(s) => ("sha256", s.into_owned()),
            ImageChecksum::SHA512(s) => ("sha512", s.into_owned()),
        }
    }
}

/// Instance-specific information.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct InstanceInfo(pub HashMap<String, Value>);

impl Deref for InstanceInfo {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InstanceInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl InstanceInfo {
    #[inline]
    fn subfield(&self, key: &str) -> Option<&String> {
        if let Some(Value::String(s)) = self.0.get(key) {
            Some(s)
        } else {
            None
        }
    }

    info_string_field! {
        #[doc = "ISO image to boot in case of the ramdisk deploy"]
        boot_iso, set_boot_iso, with_boot_iso -> "boot_iso"
    }

    /// Checksum of the image (if the algorithm is supported)
    pub fn image_checksum(&self) -> Option<ImageChecksum<'_>> {
        match self.0.get("image_os_hash_algo") {
            Some(Value::String(algo)) => {
                let value = Cow::Borrowed(self.subfield("image_os_hash_value")?.as_str());
                match algo.as_str() {
                    "md5" => Some(ImageChecksum::MD5(value)),
                    "sha256" => Some(ImageChecksum::SHA256(value)),
                    "sha512" => Some(ImageChecksum::SHA512(value)),
                    _ => None,
                }
            }
            _ => {
                let value = Cow::Borrowed(self.subfield("image_checksum")?.as_str());
                Some(ImageChecksum::MD5(value))
            }
        }
    }

    /// Checksum of the image (if the algorithm is supported)
    #[allow(unused_results)]
    pub fn set_image_checksum<'c>(&mut self, value: ImageChecksum<'c>) {
        let (algo, value) = value.into_type_value();
        match algo {
            "md5" => {
                self.0.insert("image_checksum".into(), Value::String(value));
                self.0.remove("image_os_hash_algo");
                self.0.remove("image_os_hash_value");
            }
            _ => {
                self.0
                    .insert("image_os_hash_algo".into(), Value::String(algo.into()));
                self.0
                    .insert("image_os_hash_value".into(), Value::String(value));
                self.0.remove("image_checksum");
            }
        }
    }

    /// Checksum of the image (if the algorithm is supported)
    pub fn with_image_checksum<'c>(mut self, value: ImageChecksum<'c>) -> Self {
        self.set_image_checksum(value);
        self
    }

    info_string_field! {
        #[doc = "Image to write to disk in case of a normal deployment"]
        image_source, set_image_source, with_image_source -> "image_source"
    }
}

/// Node properties.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Properties(pub HashMap<String, Value>);

impl Deref for Properties {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Properties {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod test {
    mod instance_info {
        use serde_json::Value;

        use super::super::*;

        #[test]
        fn test_empty() {
            let ii = InstanceInfo::default();
            assert!(ii.is_empty());
            assert!(ii.image_checksum().is_none());
            assert!(ii.image_source().is_none());
        }

        #[test]
        fn test_read_md5() {
            let mut ii = InstanceInfo::default();
            let _ = ii.insert("image_source".into(), Value::String("http://url".into()));
            let _ = ii.insert("image_checksum".into(), Value::String("abcd".into()));
            let cs = ii.image_checksum();
            if let Some(ImageChecksum::MD5(cs)) = cs {
                assert_eq!(&cs, "abcd");
            } else {
                panic!("Unexpected {cs:?}");
            }
            assert_eq!(ii.image_source().unwrap(), "http://url");
        }

        #[test]
        fn test_read_sha512() {
            let mut ii = InstanceInfo::default();
            let _ = ii.insert("image_source".into(), Value::String("http://url".into()));
            let _ = ii.insert("image_os_hash_value".into(), Value::String("abcd".into()));
            let _ = ii.insert("image_os_hash_algo".into(), Value::String("sha512".into()));
            let cs = ii.image_checksum();
            if let Some(ImageChecksum::SHA512(cs)) = cs {
                assert_eq!(&cs, "abcd");
            } else {
                panic!("Unexpected {cs:?}");
            }
            assert_eq!(ii.image_source().unwrap(), "http://url");
        }

        #[test]
        fn test_write_md5() {
            let cs = ImageChecksum::MD5("abcd".into());
            let ii = InstanceInfo::default()
                .with_image_checksum(cs)
                .with_image_source("http://url");
            assert_eq!(
                *ii.get("image_source").unwrap(),
                Value::String("http://url".into())
            );
            assert_eq!(
                *ii.get("image_checksum").unwrap(),
                Value::String("abcd".into())
            );
            assert!(!ii.contains_key("image_os_hash_algo"));
            assert!(!ii.contains_key("image_os_hash_value"));
        }

        #[test]
        fn test_write_sha512() {
            let cs = ImageChecksum::SHA512("abcd".into());
            let ii = InstanceInfo::default()
                .with_image_checksum(cs)
                .with_image_source("http://url");
            assert_eq!(
                *ii.get("image_source").unwrap(),
                Value::String("http://url".into())
            );
            assert_eq!(
                *ii.get("image_os_hash_value").unwrap(),
                Value::String("abcd".into())
            );
            assert_eq!(
                *ii.get("image_os_hash_algo").unwrap(),
                Value::String("sha512".into())
            );
            assert!(!ii.contains_key("image_checksum"));
        }

        #[test]
        fn test_replace_md5_with_sha512() {
            let mut ii = InstanceInfo::default();
            let _ = ii.insert("image_checksum".into(), Value::String("abcd".into()));
            let cs = ImageChecksum::SHA512("abcd".into());
            ii.set_image_checksum(cs);

            let cs = ii.image_checksum();
            if let Some(ImageChecksum::SHA512(cs)) = cs {
                assert_eq!(&cs, "abcd");
            } else {
                panic!("Unexpected {cs:?}");
            }

            assert!(!ii.contains_key("image_checksum"));
        }
    }
}
