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

//! Types shared between services and used for conversion.

use std::fmt;

macro_rules! opaque_resource_type {
    ($(#[$attr:meta])* $name:ident) => (
        $(#[$attr])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(String);

        impl From<String> for $name {
            fn from(value: String) -> $name {
                $name(value)
            }
        }

        impl<'s> From<&'s str> for $name {
            fn from(value: &'s str) -> $name {
                $name(String::from(value))
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )
}

opaque_resource_type!(#[doc = "An ID of a `Flavor`"] FlavorId);

opaque_resource_type!(#[doc = "An ID of an `Image`"] ImageId);

opaque_resource_type!(#[doc = "An ID of a `Project`"] ProjectId);

opaque_resource_type!(#[doc = "An ID of a `User`"] UserId);
