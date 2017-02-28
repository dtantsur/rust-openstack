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

#[macro_use]
extern crate hyper;
#[cfg(feature = "tls")]
extern crate hyper_rustls;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mime;
extern crate rustc_serialize;
extern crate time;

pub mod auth;
pub mod session;
pub mod utils;
