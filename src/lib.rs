// Copyright 2017-2022 Dmitry Tantsur <divius.inside@gmail.com>
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
//! async fn get_server_uuids() -> openstack::Result<Vec<String>> {
//!     let os = openstack::Cloud::from_env().await?;
//!     let server_names = os
//!         .list_servers()
//!         .await?
//!         .into_iter()
//!         .map(|server| server.id().clone())
//!         .collect();
//!     Ok(server_names)
//! }
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() { get_server_uuids().await.unwrap(); }
//! ```
//!
//! ## Find images
//!
//! Find public images using Identity password authentication with the default region:
//!
//! ```rust,no_run
//! use futures::TryStreamExt;
//!
//! async fn get_public_image_names() -> openstack::Result<Vec<String>> {
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
//!     let os = openstack::Cloud::new(auth).await?;
//!     let image_names = os
//!         .find_images()
//!         .with_visibility(openstack::image::ImageVisibility::Public)
//!         .into_stream()
//!         // This `map_ok` comes from `futures::TryStreamExt`, thus the closure returns a `Future`.
//!         .map_ok(|image| image.name().clone())
//!         .try_collect()
//!         .await?;
//!     Ok(image_names)
//! }
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() { get_public_image_names().await.unwrap(); }
//! ```
//!
//! Notice the difference between `list_*` methods (return a result with a vector) and `find_*`
//! methods (return a query builder that can be used to create a stream).
//!
//! ## Create server
//!
//! Create a server with authentication from a `clouds.yaml` file:
//!
//! ```rust,no_run
//! use openstack::waiter::Waiter;
//!
//! async fn create_server() -> openstack::Result<openstack::compute::Server> {
//!     openstack::Cloud::from_config("my-cloud-1")
//!         .await?
//!         .new_server("test-server-1", "x-large")
//!         .with_image("centos-7")
//!         .with_network("private")
//!         .with_keypair("default")
//!         .create()
//!         .await?
//!         .wait()
//!         .await
//! }
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() { create_server().await.unwrap(); }
//! ```
//!
//! # Requirements
//!
//! This crate requires Rust 2022 edition and rustc version 1.76.0 or newer.

#![crate_name = "openstack"]
#![crate_type = "lib"]
#![doc(html_root_url = "https://docs.rs/openstack/0.6.0")]
// NOTE: we do not use generic deny(warnings) to avoid breakages with new
// versions of the compiler. Add more warnings here as you discover them.
// Taken from https://github.com/rust-unofficial/patterns/
#![deny(
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
    // FIXME(dtantsur): https://github.com/rust-lang/rust/issues/122533
    // unused_qualifications,
    unused_results,
    while_true
)]
// TODO(dtantsur): revise these
#![allow(unused_extern_crates)]
#![allow(unused_macro_rules)]
#![allow(
    clippy::new_ret_no_self,
    clippy::should_implement_trait,
    clippy::wrong_self_convention
)]

#[macro_use]
extern crate log;

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
macro_rules! query_filter_ng {
    ($(#[$attr:meta])* $func:ident -> $name:path) => (
        $(#[$attr])*
        pub fn $func<T: Into<String>>(mut self, value: T) -> Self {
            self.query.push($name(value.into()));
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:path) => (
        $(#[$attr])*
        pub fn $set_func<T: Into<String>>(&mut self, value: T)  {
            self.query.push($name(value.into()));
        }

        $(#[$attr])*
        #[inline]
        pub fn $with_func<T: Into<String>>(mut self, value: T) -> Self {
            self.$set_func(value);
            self
        }
    );

    ($(#[$attr:meta])* $func:ident -> $name:path: $type:ty) => (
        $(#[$attr])*
        pub fn $func<T: Into<$type>>(mut self, value: T) -> Self {
            self.query.push($name(value.into()));
            self
        }
    );

    ($(#[$attr:meta])* $set_func:ident, $with_func:ident -> $name:path: $type:ty) => (
        $(#[$attr])*
        pub fn $set_func<T: Into<$type>>(&mut self, value: T)  {
            self.query.push($name(value.into()));
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
        #[non_exhaustive]
        pub enum $name {
            $($(#[$iattr])* $item),+,
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
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                    where S: ::serde::ser::Serializer {
                match self {
                    $($name::$item => $val),+,
                }.serialize(serializer)
            }
        }

        impl From<$name> for $carrier {
            fn from(value: $name) -> $carrier {
                match value {
                    $($name::$item => $val),+,
                }
            }
        }
    );

    {$(#[$attr:meta])* enum $name:ident {
        $($(#[$iattr:meta])* $item:ident = $val:expr),+
    }} => (
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $name {
            $($(#[$iattr])* $item),+,
        }

        impl $name {
            fn as_ref(&self) -> &'static str {
                match *self {
                    $($name::$item => $val),+,
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

    {$(#[$attr:meta])* enum $name:ident = $def:ident {
        $($(#[$iattr:meta])* $item:ident = $val:expr),+
    }} => (
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $name {
            $($(#[$iattr])* $item),+,
        }

        impl $name {
            fn as_ref(&self) -> &'static str {
                match *self {
                    $($name::$item => $val),+,
                }
            }
        }

        impl<'de> ::serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                    where D: ::serde::de::Deserializer<'de> {
                Ok(match String::deserialize(deserializer)?.as_str() {
                    $($val => $name::$item),+,
                    _ => $name::$def,
                })
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
                if *self == $name::$def {
                        use ::serde::ser::Error;
                        let err = format!("Cannot serialize default value {}::{}",
                                          stringify!($name), stringify!($def));
                        return Err(S::Error::custom(err));
                }

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
    pub use osauth::identity::{Password, Scope, Token};
    pub use osauth::{AuthType, NoAuth};
}
#[cfg(feature = "baremetal")]
pub mod baremetal;
#[cfg(feature = "block-storage")]
pub mod block_storage;
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
/// Synchronous sessions based on one from [osauth](https://docs.rs/osauth/).
pub mod session {
    pub use osauth::services::ServiceType;
    pub use osauth::Session;
}
mod utils;
pub mod waiter;

pub use osauth::common::IdOrName;
pub use osauth::{EndpointFilters, Error, ErrorKind, InterfaceType, ValidInterfaces};

/// A result of an OpenStack operation.
pub type Result<T> = std::result::Result<T, Error>;

pub use crate::cloud::Cloud;
pub use crate::common::Refresh;

/// Sorting request.
#[derive(Debug, Clone)]
pub enum Sort<T> {
    /// Sorting by given field in ascendant order.
    Asc(T),
    /// Sorting by given field in descendant order.
    Desc(T),
}

impl<T> Sort<T> {
    #[allow(unused)]
    fn unwrap(self) -> (T, utils::SortDir) {
        match self {
            Sort::Asc(val) => (val, utils::SortDir::Asc),
            Sort::Desc(val) => (val, utils::SortDir::Desc),
        }
    }
}

impl<T: Into<String>> From<Sort<T>> for (String, String) {
    fn from(other: Sort<T>) -> (String, String) {
        match other {
            Sort::Asc(val) => (val.into(), String::from("asc")),
            Sort::Desc(val) => (val.into(), String::from("desc")),
        }
    }
}
