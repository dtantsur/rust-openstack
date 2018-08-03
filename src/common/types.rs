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

//! Types and traits shared between services.

use std::rc::Rc;

use serde::Serialize;

use super::super::Result;
use super::super::session::Session;


/// Trait representing something that can be listed from a session.
pub trait ListResources {
    /// Default limit to use with this resource.
    const DEFAULT_LIMIT: usize;

    /// Whether pagination is supported for this resource.
    fn can_paginate(_session: &Session) -> Result<bool> { Ok(true) }

    /// List the resources from the session.
    fn list_resources<Q>(session: Rc<Session>, query: Q) -> Result<Vec<Self>>
        where Self: Sized, Q: Serialize + ::std::fmt::Debug;
}

/// Trait representing something that can be refreshed.
pub trait Refresh {
    /// Refresh the resource representation.
    fn refresh(&mut self) -> Result<()>;
}

/// Trait representing something that has an ID.
pub trait ResourceId {
    /// Identifier of the current resource.
    fn resource_id(&self) -> String;
}

macro_rules! opaque_resource_type {
    ($(#[$attr:meta])* $name:ident ? $service:expr) => (
        $(#[$attr])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub(crate) value: String,
            pub(crate) verified: bool
        }

        impl From<String> for $name {
            fn from(value: String) -> $name {
                $name {
                    value: value,
                    verified: false
                }
            }
        }

        impl<'s> From<&'s str> for $name {
            fn from(value: &'s str) -> $name {
                $name {
                    value: String::from(value),
                    verified: false
                }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.value
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.value
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                self.value.fmt(f)
            }
        }

        impl ::serde::ser::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                    where S: ::serde::ser::Serializer {
                serializer.serialize_str(&self.value)
            }
        }

        impl<'de> ::serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D)
                    -> ::std::result::Result<$name, D::Error>
                    where D: ::serde::de::Deserializer<'de> {
                Ok($name {
                    value: String::deserialize(deserializer)?,
                    // Assume that values coming from network are valid.
                    verified: true
                })
            }
        }

        impl $name {
            /// Create a reference that was previously verified.
            #[allow(dead_code)]
            pub(crate) fn new_verified(value: String) -> $name {
                $name {
                    value: value,
                    verified: true
                }
            }

            /// Verify this reference and convert to an ID, if possible.
            #[cfg(not(feature = $service))]
            #[allow(dead_code)]
            pub(crate) fn into_verified(self, _session: &$crate::session::Session)
                    -> $crate::Result<String> {
                Ok(self.value)
            }
        }
    )
}

opaque_resource_type!(#[doc = "An ID of a `Flavor`"] FlavorRef ? "compute");

opaque_resource_type!(#[doc = "An ID of an `Image`"] ImageRef ? "image");

opaque_resource_type!(#[doc = "An ID of a `KeyPair`"] KeyPairRef ? "compute");

opaque_resource_type!(#[doc = "An ID of a `Network`"] NetworkRef ? "network");

opaque_resource_type!(#[doc = "An ID of a `Project`"] ProjectRef ? "identity");

opaque_resource_type!(#[doc = "An ID of a `Port`"] PortRef ? "network");

opaque_resource_type!(#[doc = "An ID of a `Subnet`"] SubnetRef ? "network");

opaque_resource_type!(#[doc = "An ID of a `User`"] UserRef ? "identity");


#[cfg(test)]
mod test {
    use serde_json;

    opaque_resource_type!(TestId ? "test");

    #[test]
    fn test_opaque_type_basics() {
        let id = TestId::from("foo");
        assert_eq!(id.as_ref(), "foo");
        assert_eq!(&id.to_string(), "foo");
        assert_eq!(id, TestId::from("foo"));
        assert!(id != TestId::from("bar"));
        let s: String = id.into();
        assert_eq!(&s, "foo");
    }

    #[test]
    fn test_opaque_type_serde() {
        let id: TestId = serde_json::from_str("\"foo\"").unwrap();
        assert_eq!(id.as_ref(), "foo");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"foo\"");
    }
}
