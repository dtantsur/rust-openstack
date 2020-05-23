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
//! [Cloud](struct.Cloud.html) object and use it for OpenStack API calls.
//!
//! # Examples
//!
//! ## List servers
//!
//! Get authentication parameters from the environment and get UUIDs of all
//! servers.
//!
//! ```rust,no_run
//! extern crate openstack;
//!
//! fn get_server_uuids() -> openstack::Result<Vec<String>> {
//!     let os = openstack::Cloud::from_env()?;
//!     let server_names = os
//!         .list_servers()?
//!         .into_iter()
//!         .map(|server| server.id().clone())
//!         .collect();
//!     Ok(server_names)
//! }
//! # fn main() { get_server_uuids().unwrap(); }
//! ```
//!
//! ## Find images
//!
//! Find public images using Identity password authentication with the default region:
//!
//! ```rust,no_run
//! extern crate fallible_iterator;
//! extern crate openstack;
//!
//! use fallible_iterator::FallibleIterator;
//!
//! fn get_public_image_names() -> openstack::Result<Vec<String>> {
//!     let scope = openstack::auth::Scope::Project {
//!         project: openstack::IdOrName::from_name("project1"),
//!         domain: Some(openstack::IdOrName::from_id("default")),
//!     };
//!     let auth = openstack::auth::Password::new(
//!         "https://cloud.local/identity",
//!         "admin",
//!         "pa$$w0rd",
//!         "Default"
//!     )
//!     .expect("Invalid auth_url")
//!     .with_scope(scope);
//!
//!     let os = openstack::Cloud::new(auth);
//!     let image_names = os
//!         .find_images()
//!         .with_visibility(openstack::image::ImageVisibility::Public)
//!         .into_iter()
//!         // This `map` comes from fallible-iterator, thus the closure returns a `Result`.
//!         .map(|image| Ok(image.name().clone()))
//!         .collect()?;
//!     Ok(image_names)
//! }
//! # fn main() { get_public_image_names().unwrap(); }
//! ```
//!
//! Notice the difference between `list_*` methods (return a result with a vector) and `find_*`
//! methods (return a query builder that can be used to create a fallible iterator).
//!
//! ## Create server
//!
//! Create a server with authentication from a `clouds.yaml` file:
//!
//! ```rust,no_run
//! extern crate openstack;
//! extern crate waiter;
//!
//! // Required for the `wait` call.
//! use waiter::Waiter;
//!
//! fn create_server() -> openstack::Result<openstack::compute::Server> {
//!     openstack::Cloud::from_config("my-cloud-1")?
//!         .new_server("test-server-1", "x-large")
//!         .with_image("centos-7")
//!         .with_network("private")
//!         .with_keypair("default")
//!         .create()?
//!         .wait()
//! }
//! # fn main() { create_server().unwrap(); }
//! ```

#![crate_name = "openstack"]
#![crate_type = "lib"]
#![doc(html_root_url = "https://docs.rs/openstack/0.4.0")]
// NOTE: we do not use generic deny(warnings) to avoid breakages with new
// versions of the compiler. Add more warnings here as you discover them.
// Taken from https://github.com/rust-unofficial/patterns/
#![deny(
    const_err,
    dead_code,
    improper_ctypes,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    trivial_casts,
    trivial_numeric_casts,
    unconditional_recursion,
    unsafe_code,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_doc_comments,
    unused_import_braces,
    unused_parens,
    unused_qualifications,
    unused_results,
    while_true
)]
#![allow(unused_extern_crates)]
#![allow(
    clippy::new_ret_no_self,
    clippy::should_implement_trait,
    clippy::wrong_self_convention
)]

extern crate chrono;
extern crate eui48;
extern crate fallible_iterator;
extern crate ipnet;
#[macro_use]
extern crate log;
extern crate osauth;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
extern crate waiter;

#[allow(unused_macros)]
macro_rules! transparent_property {
    ($(#[$attr:meta])* $name:ident: ref $type:ty) => (
        $(#[$attr])*
        #[inline]
        pub fn $name(&self) -> &$type {
            &self.inner.$name
        }
    );

    ($(#[$attr:meta])* $name:ident: $type:ty) => (
        $(#[$attr])*
        #[inline]
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

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident) => (
        $(#[$attr])*
        pub fn $set_func<T: Into<String>>(&mut self, value: T)  {
            self.query.push_str(stringify!($name), value);
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<T: Into<String>>(mut self, value: T) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        pub fn $func<T: Into<$type>>(mut self, value: T) -> Self {
            self.query.push(stringify!($name), value.into());
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        pub fn $set_func<T: Into<$type>>(&mut self, value: T)  {
            self.query.push(stringify!($name), value.into());
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<T: Into<$type>>(mut self, value: T) -> Self {
            self.$set_func(value.into());
            self
        }
    );
}

#[allow(unused_macros)]
macro_rules! creation_field {

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func<S: Into<String>>(&mut self, value: S)  {
            self.$name = value.into();
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func(&mut self, value: $type)  {
            self.$name = value;
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: optional String) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func<S: Into<String>>(&mut self, value: S)  {
            self.$name = Some(value.into());
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: optional $type:ty) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func(&mut self, value: $type)  {
            self.$name = Some(value);
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

}

#[allow(unused_macros)]
macro_rules! creation_inner_field {

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func<S: Into<String>>(&mut self, value: S)  {
            self.inner.$name = value.into();
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func(&mut self, value: $type)  {
            self.inner.$name = value;
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: optional String) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func<S: Into<String>>(&mut self, value: S)  {
            self.inner.$name = Some(value.into());
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: optional $type:ty) => (
        $(#[$attr])*
        #[inline]
        pub fn $set_func(&mut self, value: $type)  {
            self.inner.$name = Some(value);
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

}

#[allow(unused_macros)]
macro_rules! creation_inner_vec {

    ($(#[$attr:meta])* $add_func:ident, $with_func:ident -> $name:ident) => (
        $(#[$attr])*
        pub fn $add_func<S: Into<String>>(&mut self, value: S)  {
            self.inner.$name.push(value.into());
        }

        $(#[$attr])*
        #[inline]
        pub fn $name(&mut self) -> &mut Vec<String> {
            &mut self.inner.$name
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$add_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $add_func:ident, $with_func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        pub fn $add_func(&mut self, value: $type)  {
            self.inner.$name.push(value);
        }

        $(#[$attr])*
        #[inline]
        pub fn $name(&mut self) -> &mut Vec<$type> {
            &mut self.inner.$name
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$add_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $add_func:ident, $with_func:ident -> $name:ident: into $type:ty) => (
        $(#[$attr])*
        pub fn $add_func<S: Into<$type>>(&mut self, value: S)  {
            self.inner.$name.push(value.into());
        }

        $(#[$attr])*
        #[inline]
        pub fn $name(&mut self) -> &mut Vec<$type> {
            &mut self.inner.$name
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<$type>>(mut self, value: S) -> Self {
            self.$add_func(value);
            self
        }
    );


}

#[allow(unused_macros)]
macro_rules! update_field {

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident) => (
        $(#[$attr])*
        pub fn $set_func<S: Into<String>>(&mut self, value: S)  {
            self.inner.$name = value.into();
            self.dirty.insert(stringify!($name));
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        #[allow(unused_results)]
        pub fn $set_func(&mut self, value: $type)  {
            self.inner.$name = value;
            self.dirty.insert(stringify!($name));
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: optional String) => (
        $(#[$attr])*
        #[allow(unused_results)]
        pub fn $set_func<S: Into<String>>(&mut self, value: S)  {
            self.inner.$name = Some(value.into());
            self.dirty.insert(stringify!($name));
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<S: Into<String>>(mut self, value: S) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:ident: optional $type:ty) => (
        $(#[$attr])*
        #[allow(unused_results)]
        pub fn $set_func(&mut self, value: $type)  {
            self.inner.$name = Some(value);
            self.dirty.insert(stringify!($name));
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

}

#[allow(unused_macros)]
macro_rules! update_field_mut {

    ($(#[$attr:meta])* $mut_func:ident, $set_func:ident, $with_func:ident -> $name:ident: $type:ty) => (
        $(#[$attr])*
        #[allow(unused_results)]
        pub fn $mut_func(&mut self) -> &mut $type {
            self.dirty.insert(stringify!($name));
            &mut self.inner.$name
        }

        $(#[$attr])*
        #[allow(unused_results)]
        pub fn $set_func(&mut self, value: $type)  {
            self.inner.$name = value;
            self.dirty.insert(stringify!($name));
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func(mut self, value: $type) -> Self {
            self.$set_func(value);
            self
        }
    );

}

#[allow(unused_macros)]
macro_rules! save_option_fields {
    ($self:ident -> $target:ident: $($field:ident)+) => {
        $($target.$field = if $self.dirty.contains(stringify!($field)) {
            $self.inner.$field.clone()
        } else {
            None
        };)+
    }
}

#[allow(unused_macros)]
macro_rules! save_fields {
    ($self:ident -> $target:ident: $($field:ident)+) => {
        $($target.$field = if $self.dirty.contains(stringify!($field)) {
            Some($self.inner.$field.clone())
        } else {
            None
        };)+
    }
}

#[allow(unused_macros)]
macro_rules! protocol_enum {
    {$(#[$attr:meta])* enum $name:ident: $carrier:ty {
        $($(#[$iattr:meta])* $item:ident = $val:expr),+
    }} => (
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($(#[$iattr])* $item),+,
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

        impl ::serde::ser::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where S: ::serde::ser::Serializer {
                match self {
                    $($name::$item => $val),+,
                    _ => unreachable!()
                }.serialize(serializer)
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
        $($(#[$iattr:meta])* $item:ident = $val:expr),+
    }} => (
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($(#[$iattr])* $item),+,
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
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
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

/// Reimports of authentication bits from `osauth`.
///
/// See [osauth documentation](https://docs.rs/osauth/) for details.
pub mod auth {
    pub use osauth::identity::{Identity, Password, Scope};
    pub use osauth::{from_config, from_env, AuthType, NoAuth};
}
mod cloud;
pub mod common;
#[cfg(feature = "compute")]
pub mod compute;
#[cfg(feature = "image")]
pub mod image;
#[cfg(feature = "network")]
pub mod network;
#[cfg(feature = "object-storage")]
pub mod object_storage;
/// Reimport of the synchronous session from `osauth`.
///
/// See [osauth documentation](https://docs.rs/osauth/) for details.
pub mod session {
    pub use osauth::services::ServiceType;
    pub use osauth::sync::SyncSession as Session;
}
mod utils;

pub use osauth::identity::IdOrName;
pub use osauth::sync::Result;
pub use osauth::{EndpointFilters, Error, ErrorKind, InterfaceType, ValidInterfaces};

pub use crate::cloud::Cloud;
pub use crate::common::Refresh;

/// Sorting request.
#[derive(Debug, Clone)]
pub enum Sort<T: Into<String>> {
    /// Sorting by given field in ascendant order.
    Asc(T),
    /// Sorting by given field in descendant order.
    Desc(T),
}

impl<T: Into<String>> Into<(String, String)> for Sort<T> {
    fn into(self) -> (String, String) {
        match self {
            Sort::Asc(val) => (val.into(), String::from("asc")),
            Sort::Desc(val) => (val.into(), String::from("desc")),
        }
    }
}
