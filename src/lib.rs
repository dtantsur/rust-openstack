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

//! OpenStack Client in Rust.
//!
//! The goal of this project is to provide a simple API for working with
//! OpenStack clouds.

#![crate_name = "openstack"]
#![crate_type = "lib"]
#![warn(missing_docs,
        missing_debug_implementations,
        missing_copy_implementations,
        trivial_casts,
        trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces,
        unused_qualifications)]

#[macro_use]
extern crate hyper;
#[cfg(feature = "tls")]
extern crate hyper_rustls;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate time;

#[cfg(test)] #[macro_use]
extern crate yup_hyper_mock;

pub mod auth;
mod common;
pub mod identity;
pub mod session;
pub mod utils;

pub use common::ApiError;
