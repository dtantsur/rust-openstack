// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
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

//! OpenStack SDK in Rust.
//!
//! The goal of this project is to provide a simple API for working with
//! OpenStack clouds.
//!
//! # Usage
//!
//! Start with [authentication](auth/index.html), then create a
//! [Cloud](struct.Cloud.html) object that contains all necessary calls.
//!
//! # API
//!
//! * [Compute API](compute/index.html)
//! * [Image API](image/index.html)
//! * [Network API](network/index.html)

#![crate_name = "openstack"]
#![crate_type = "lib"]
// NOTE: we do not use generic deny(warnings) to avoid breakages with new
// versions of the compiler. Add more warnings here as you discover them.
// Taken from https://github.com/rust-unofficial/patterns/
#![deny(const_err,
        dead_code,
        improper_ctypes,
        legacy_directory_ownership,
        missing_copy_implementations,
        missing_debug_implementations,
        missing_docs,
        non_shorthand_field_patterns,
        no_mangle_generic_items,
        overflowing_literals,
        path_statements ,
        patterns_in_fns_without_body,
        plugin_as_library,
        private_in_public,
        private_no_mangle_fns,
        private_no_mangle_statics,
        safe_extern_statics,
        trivial_casts,
        trivial_numeric_casts,
        unconditional_recursion,
        unions_with_drop_fields,
        unsafe_code,
        unused,
        unused_allocation,
        unused_comparisons,
        unused_doc_comment,
        unused_extern_crates,
        unused_import_braces,
        unused_parens,
        unused_qualifications,
        unused_results,
        while_true)]

#[allow(unused_extern_crates)]
extern crate chrono;
#[allow(unused_extern_crates)]
extern crate fallible_iterator;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[allow(unused_extern_crates)]
extern crate serde_json;
extern crate waiter;


#[allow(unused_macros)]
macro_rules! transparent_property {
    ($(#[$attr:meta])* $name:ident: ref $type:ty) => (
        $(#[$attr])*
        pub fn $name(&self) -> &$type {
            &self.inner.$name
        }
    );

    ($(#[$attr:meta])* $name:ident: $type:ty) => (
        $(#[$attr])*
        pub fn $name(&self) -> $type {
            self.inner.$name
        }
    );
}


#[allow(unused_macros)]
macro_rules! query_filter {
    ($(#[$attr:meta])* $func:ident -> $name:ident) => (
        $(#[$attr])*
        pub fn $func<T: Into<String>>(mut self, value: T) -> Self {
            self.query.push_str(stringify!($name), value);
            self
        }
    );

    ($(#[$attr:meta])* $func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        pub fn $func(mut self, value: $type) -> Self {
            self.query.push(stringify!($name), value);
            self
        }
    );
}


#[allow(unused_macros)]
macro_rules! protocol_enum {
    {$(#[$attr:meta])* enum $name:ident: $carrier:ty {
        $($item:ident = $val:expr),+
    }} => (
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($item),+,
            #[doc(hidden)]
            __Nonexhaustive,
        }

        impl<'de> ::serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                    where D: ::serde::de::Deserializer<'de> {
                let value: $carrier = ::serde::de::Deserialize::deserialize(
                    deserializer)?;
                match value {
                    $($val => Ok($name::$item)),+,
                    other => {
                        use ::serde::de::Error;
                        let err = format!("Unexpected {}: {}",
                                          stringify!($name), other);
                        Err(D::Error::custom(err))
                    }
                }
            }
        }

        impl From<$name> for $carrier {
            fn from(value: $name) -> $carrier {
                match value {
                    $($name::$item => $val),+,
                    _ => unreachable!()
                }
            }
        }
    );

    {$(#[$attr:meta])* enum $name:ident {
        $($item:ident = $val:expr),+
    }} => (
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($item),+,
            #[doc(hidden)]
            __Nonexhaustive,
        }

        impl $name {
            fn as_ref(&self) -> &'static str {
                match *self {
                    $($name::$item => $val),+,
                    _ => unreachable!()
                }
            }
        }

        impl<'de> ::serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                    where D: ::serde::de::Deserializer<'de> {
                match String::deserialize(deserializer)?.as_ref() {
                    $($val => Ok($name::$item)),+,
                    other => {
                        use ::serde::de::Error;
                        let err = format!("Unexpected {}: {}",
                                          stringify!($name), other);
                        Err(D::Error::custom(err))
                    }
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(self.as_ref())
            }
        }

        impl ::serde::ser::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where S: ::serde::ser::Serializer {
                serializer.serialize_str(self.as_ref())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                String::from(value.as_ref())
            }
        }
    );
}


pub mod auth;
mod cloud;
pub mod common;
#[cfg(feature = "compute")]
pub mod compute;
mod error;
mod identity;
#[cfg(feature = "image")]
pub mod image;
#[cfg(feature = "network")]
pub mod network;
pub mod session;
mod utils;

pub use cloud::Cloud;
pub use common::Refresh;
pub use error::{Error, ErrorKind, Result};


/// Sorting request.
#[derive(Debug, Clone)]
pub enum Sort<T: Into<String>> {
    /// Sorting by given field in ascendant order.
    Asc(T),
    /// Sorting by given field in descendant order.
    Desc(T)
}

impl<T: Into<String>> Into<(String, String)> for Sort<T> {
    fn into(self) -> (String, String) {
        match self {
            Sort::Asc(val) => (val.into(), String::from("asc")),
            Sort::Desc(val) => (val.into(), String::from("desc"))
        }
    }
}
