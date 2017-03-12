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
//!
//! # Features
//!
//! * [Authentication](auth/index.html)
//! * Basic [compute API support](compute/index.html)

#![crate_name = "openstack"]
#![crate_type = "lib"]
// NOTE: we do not use generic deny(warnings) to avoid breakages with new
// versions of the compiler. Add more warnings here as you discover them.
// Taken from https://github.com/rust-unofficial/patterns/
#![deny(const_err,
        dead_code,
        extra_requirement_in_impl,
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
        unused_extern_crates,
        unused_import_braces,
        unused_parens,
        unused_qualifications,
        unused_results,
        while_true)]

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
#[cfg(feature = "compute")]
pub mod compute;
pub mod http;
mod identity;
pub mod service;
mod session;
mod utils;

pub use common::ApiError;
pub use common::ApiResult;
pub use common::ApiVersion;
pub use session::Session;
